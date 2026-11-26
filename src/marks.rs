//! Marks, on the server: a press places or takes back a mark, every player in its dimension is
//! sent the beams, and the name floats above it as a vanilla text display for 30 seconds.
use crate::payloads;
use crate::rules::{self, Mark, Marks, Pos, Pressed};
use mod_api::{Compound, Host, PlayerRef, ServerHost, Tag, VarInt};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Every name display carries this tag, so one left from a stopped server is known and removed.
const LABEL_TAG: &str = "share.mark";

#[derive(Default)]
struct State {
    marks: Marks,
    /// The dimension each player was last sent the marks of.
    sent: HashMap<u128, String>,
    /// The text displays this server summoned and still shows.
    labels: HashSet<u32>,
    /// True while summoning a label (its join may come inside the call).
    summoning: bool,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(STATE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(State::default))
}

fn on(host: &dyn Host) -> bool {
    crate::enabled(host, crate::setting::MARKS)
}

fn dimension(host: &dyn Host, p: PlayerRef) -> Option<String> {
    p.dimension(host).map(|d| d.to_string())
}

/// A player pressed the mark key looking at `pos`.
pub fn press(host: &mut dyn ServerHost, p: PlayerRef, pos: Pos) {
    if !on(&*host) {
        return;
    }
    let read: &dyn Host = &*host;
    let (Some(dim), Some(eye), Some(name)) = (dimension(read, p), p.entity().eye_position(read), p.name(read)) else { return };
    let centre = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    let far = (0..3).map(|i| (eye[i] - centre[i]).powi(2)).sum::<f64>().sqrt();
    if far > f64::from(rules::REACH) + 2.0 {
        return;
    }
    let now = crate::world::now();
    let pressed = with(|s| s.marks.press(p.uuid(), &name, &dim, pos, now));
    match pressed {
        Pressed::Removed(old) => unlabel(host, &old),
        Pressed::Placed { replaced } => {
            if let Some(old) = replaced {
                unlabel(host, &old);
            }
            label(host, p.uuid(), &name, &dim, pos);
        }
    }
    send_all(host);
}

/// Summons the floating name over the newest mark at `pos`.
fn label(host: &mut dyn ServerHost, owner: u128, name: &str, dim: &str, pos: Pos) {
    let colour = format!("#{:06X}", rules::colour(owner));
    let text: Compound = [("text".into(), Tag::String(name.into())), ("color".into(), Tag::String(colour.as_str().into()))].into_iter().collect();
    let floats = |v: [f32; 4]| Tag::List(v.iter().map(|f| Tag::Float(f.to_bits())).collect());
    let three = |v: f32| Tag::List([v, v, v].iter().map(|f| Tag::Float(f.to_bits())).collect());
    let transformation: Compound = [
        ("left_rotation".into(), floats([0.0, 0.0, 0.0, 1.0])),
        ("right_rotation".into(), floats([0.0, 0.0, 0.0, 1.0])),
        ("translation".into(), three(0.0)),
        ("scale".into(), three(2.0)),
    ]
    .into_iter()
    .collect();
    let nbt: Compound = [
        ("text".into(), Tag::Compound(text)),
        ("billboard".into(), Tag::String("center".into())),
        ("see_through".into(), Tag::Byte(1)),
        ("shadow".into(), Tag::Byte(1)),
        ("background".into(), Tag::Int(0x4000_0000)),
        ("transformation".into(), Tag::Compound(transformation)),
        ("Tags".into(), Tag::List(vec![Tag::String(LABEL_TAG.into())])),
    ]
    .into_iter()
    .collect();
    let at = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 2.2, f64::from(pos[2]) + 0.5];
    with(|s| s.summoning = true);
    let id = host.summon(dim, "minecraft:text_display", at, nbt);
    with(|s| {
        s.summoning = false;
        if let Some(id) = id {
            s.labels.insert(id);
            if let Some(m) = s.marks.list.iter_mut().rev().find(|m| m.owner == owner && m.pos == pos && m.dimension == dim) {
                m.label = Some(id);
            }
        }
    });
}

fn unlabel(host: &mut dyn ServerHost, mark: &Mark) {
    let Some(id) = mark.label else { return };
    with(|s| s.labels.remove(&id));
    let entity = { (&*host as &dyn Host).entity(id) };
    if let Some(e) = entity {
        e.discard(host);
    }
}

/// A text display joined a level: one of ours left from an earlier run goes.
pub fn joined(host: &mut dyn ServerHost, id: u32, kind: &str) {
    if kind != "minecraft:text_display" || with(|s| s.summoning || s.labels.contains(&id)) {
        return;
    }
    let read: &dyn Host = &*host;
    let Some(e) = read.entity(id) else { return };
    if e.tags(read).is_some_and(|t| t.iter().any(|t| t == LABEL_TAG)) {
        e.discard(host);
    }
}

/// The marks of `dim`, for its players.
fn view(dim: &str) -> payloads::Marks {
    with(|s| {
        let mut v = payloads::Marks::default();
        for m in s.marks.list.iter().filter(|m| m.dimension == dim) {
            v.at.extend(m.pos.map(VarInt));
            v.colour.push(VarInt(rules::colour(m.owner) as i32));
            v.left.push(VarInt(m.until.saturating_sub(crate::world::now()).min(i32::MAX as u64) as i32));
        }
        v
    })
}

fn send(host: &mut dyn ServerHost, p: PlayerRef) {
    let Some(token) = payloads::MARKS.get() else { return };
    let Some(dim) = dimension(&*host, p) else { return };
    match token.send(host, p, &view(&dim)) {
        Ok(_) => with(|s| s.sent.insert(p.uuid(), dim)),
        Err(e) => {
            eprintln!("[share] sending marks: {e:?}");
            with(|s| s.sent.remove(&p.uuid()))
        }
    };
}

fn send_all(host: &mut dyn ServerHost) {
    for p in host.players().server_players() {
        send(host, p);
    }
}

pub fn tick(host: &mut dyn ServerHost) {
    let now = crate::world::now();
    let gone = with(|s| s.marks.expire(now));
    for m in &gone {
        unlabel(host, m);
    }
    if !gone.is_empty() {
        send_all(host);
    } else if now.is_multiple_of(20) {
        // A player who joined or changed dimension gets that dimension's marks.
        for p in host.players().server_players() {
            let dim = dimension(&*host, p);
            if dim.is_some() && with(|s| s.sent.get(&p.uuid()) != dim.as_ref()) {
                send(host, p);
            }
        }
    }
}

pub fn forget(player: u128) {
    with(|s| s.sent.remove(&player));
}
