//! Revive instead of death: with a friend online, a player whose health runs out is down for 30 s
//! (slowed, weak, unhurt by anything but `/kill` and the void) instead of dying; a friend
//! sneaking next to them for 3 s brings them back at 4 health. Alone, or when time is up, death.
use crate::coop::{self, Down};
use crate::world;
use mod_api::{ActiveEffect, Host, LivingDeathEvent, PlayerRef, ServerHost, SoundSource, Text};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

struct Downed {
    until: u64,
    progress: u32,
}

static DOWN: Mutex<Option<HashMap<u128, Downed>>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut HashMap<u128, Downed>) -> R) -> R {
    f(DOWN.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(HashMap::new))
}

/// The effects of lying down: (effect, amplifier). Resistance 5 takes every hurt but `/kill` and
/// the void; weakness keeps their blows harmless.
const EFFECTS: [(&str, i32); 4] =
    [("minecraft:slowness", 5), ("minecraft:weakness", 9), ("minecraft:resistance", 4), ("minecraft:mining_fatigue", 4)];

pub fn is_down(player: u128) -> bool {
    with(|d| d.contains_key(&player))
}

fn effects(host: &mut dyn ServerHost, p: PlayerRef, on: bool) {
    let Some(living) = (&*host as &dyn Host).living(p.entity()) else { return };
    for (id, amp) in EFFECTS {
        let Some(key) = (&*host as &dyn Host).effect_key(id) else { continue };
        if on {
            let ticks = coop::DOWN_TICKS as i32 + 20;
            living.add_effect(host, &ActiveEffect::new(key, ticks, amp, false, false, true));
        } else {
            living.remove_effect(host, key);
        }
    }
}

fn tell_all(host: &mut dyn ServerHost, key: &str, name: &str) {
    for p in host.players().server_players() {
        let _ = p.action_bar(host, Text::translate(key, vec![Text::literal(name)]));
    }
}

/// A player's health ran out: down instead of dead, if a friend can come.
pub fn death(host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> bool {
    if event.kind != "minecraft:player" || !crate::enabled(&*host, crate::setting::REVIVE) {
        return false;
    }
    let Some(p) = world::by_id(host, event.entity) else { return false };
    let read: &dyn Host = &*host;
    let friends = host.players().server_players().into_iter().filter(|f| f.uuid() != p.uuid() && world::playing(read, *f)).count();
    if !coop::goes_down(event.source.damage_type, world::playing(read, p), is_down(p.uuid()), friends) {
        if is_down(p.uuid()) {
            with(|d| d.remove(&p.uuid()));
        }
        return false;
    }
    let Some(living) = read.living(p.entity()) else { return false };
    living.set_health(host, 1.0);
    effects(host, p, true);
    with(|d| d.insert(p.uuid(), Downed { until: world::now() + coop::DOWN_TICKS, progress: 0 }));
    let name = p.name(&*host).unwrap_or_default();
    tell_all(host, "share.down", &name);
    p.play_sound(host, "minecraft:entity.player.hurt", SoundSource::Players, 1.0, 0.6);
    true
}

/// Each tick: friends sneaking next to a downed player revive them; time up is death.
pub fn tick(host: &mut dyn ServerHost) {
    let downed: Vec<u128> = with(|d| d.keys().copied().collect());
    if downed.is_empty() {
        return;
    }
    let players = host.players().server_players();
    let now = world::now();
    for uuid in downed {
        let Some(p) = players.iter().copied().find(|x| x.uuid() == uuid) else {
            with(|d| d.remove(&uuid));
            continue;
        };
        let read: &dyn Host = &*host;
        let (Some(at), level) = (p.entity().position(read), p.level(read)) else { continue };
        let reviver = players.iter().copied().find(|f| {
            f.uuid() != uuid
                && !is_down(f.uuid())
                && world::playing(read, *f)
                && f.level(read) == level
                && f.entity().is_crouching(read) == Some(true)
                && f.entity().position(read).is_some_and(|q| world::distance(q, at) <= coop::REVIVE_REACH)
        });
        let (progress, until) = with(|d| d.get(&uuid).map(|x| (x.progress, x.until))).unwrap_or((0, now));
        let name = p.name(read).unwrap_or_default();
        match coop::down_tick(progress, reviver.is_some(), now, until) {
            Down::Waiting { progress: next } => {
                with(|d| { if let Some(x) = d.get_mut(&uuid) { x.progress = next; } });
                if let Some(f) = reviver
                    && next % 10 == 0
                {
                    let pct = next * 100 / coop::REVIVE_TICKS;
                    let line = Text::translate("share.reviving", vec![Text::literal(name.as_str()), Text::literal(format!("{pct}"))]);
                    let _ = f.action_bar(host, line.clone());
                    let _ = p.action_bar(host, line);
                } else if next == 0 && now.is_multiple_of(40) {
                    let left = until.saturating_sub(now) / 20;
                    let _ = p.action_bar(host, Text::translate("share.down.wait", vec![Text::literal(format!("{left}"))]));
                }
            }
            Down::Revived => {
                with(|d| d.remove(&uuid));
                effects(host, p, false);
                if let Some(living) = (&*host as &dyn Host).living(p.entity()) {
                    living.set_health(host, coop::REVIVED_HEALTH);
                }
                tell_all(host, "share.revived", &name);
                p.play_sound(host, "minecraft:entity.player.levelup", SoundSource::Players, 1.0, 1.2);
            }
            Down::Dies => {
                with(|d| d.remove(&uuid));
                effects(host, p, false);
                let _ = p.entity().hurt(host, "minecraft:generic_kill", f32::MAX, None, None);
            }
        }
    }
}

pub fn forget(player: u128) {
    with(|d| d.remove(&player));
}
