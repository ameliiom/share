//! The client: J sends the block you look at (up to 128 away), and each mark of your dimension
//! is drawn as a light beam in its player's colour, seen through hills, with the block outlined.
use crate::payloads;
use crate::rules::{self, Pos};
use mod_api::{ClientHost, RenderLevelStage, RenderLevelStageEvent, VarInt, WorldDepth, WorldLine, WorldQuad};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

/// How high the beam reaches above the marked block.
const BEAM: f64 = 64.0;
/// Half the beam's width.
const HALF: f64 = 0.15;

struct Shown {
    pos: Pos,
    colour: u32,
    left: i64,
    /// The game time it goes at, fixed on the first frame drawn.
    until: Option<i64>,
}

static SHOWN: Mutex<Vec<Shown>> = Mutex::new(Vec::new());

pub fn received(view: payloads::Marks) {
    let shown = view
        .at
        .as_chunks::<3>()
        .0
        .iter()
        .zip(view.colour.iter().zip(view.left.iter()))
        .map(|(at, (c, l))| Shown { pos: [at[0].0, at[1].0, at[2].0], colour: c.0 as u32 & 0xFF_FFFF, left: i64::from(l.0), until: None })
        .collect();
    *SHOWN.lock().unwrap_or_else(|e| e.into_inner()) = shown;
}

/// The mark key was pressed: send the block looked at, if one is within reach.
pub fn press(host: &mut dyn ClientHost) {
    let Some(hit) = host.raycast(rules::REACH) else { return };
    if let Some(token) = payloads::PRESS.get()
        && let Err(e) = token.send(host, &payloads::Press { at: vec![VarInt(hit.block_x), VarInt(hit.block_y), VarInt(hit.block_z)] })
    {
        eprintln!("[share] sending a mark: {e:?}");
    }
}

/// The beam's four sides, each both ways round so it shows from inside and out.
fn beam(pos: Pos, argb: u32) -> Vec<WorldQuad> {
    let (cx, cz) = (f64::from(pos[0]) + 0.5, f64::from(pos[2]) + 0.5);
    let (y0, y1) = (f64::from(pos[1]) + 1.0, f64::from(pos[1]) + 1.0 + BEAM);
    let c = [(cx - HALF, cz - HALF), (cx + HALF, cz - HALF), (cx + HALF, cz + HALF), (cx - HALF, cz + HALF)];
    (0..4)
        .flat_map(|i| {
            let (a, b) = (c[i], c[(i + 1) % 4]);
            let side = [[a.0, y0, a.1], [b.0, y0, b.1], [b.0, y1, b.1], [a.0, y1, a.1]];
            let back = [side[3], side[2], side[1], side[0]];
            [WorldQuad::new(side, argb, WorldDepth::Tested), WorldQuad::new(back, argb, WorldDepth::Tested)]
        })
        .collect()
}

/// The twelve edges of the marked block and the beam's core, seen through walls.
fn lines(pos: Pos, rgb: u32) -> Vec<WorldLine> {
    let [x, y, z] = pos.map(f64::from);
    let (lo, hi) = (-0.003, 1.003);
    let edge = 0xFF00_0000 | rgb;
    let p = |i: f64, j: f64, k: f64| [x + i, y + j, z + k];
    let corners = [(lo, lo), (hi, lo), (hi, hi), (lo, hi)];
    let mut out: Vec<WorldLine> = (0..4)
        .flat_map(|n| {
            let (a, b) = (corners[n], corners[(n + 1) % 4]);
            [
                WorldLine::new(p(a.0, lo, a.1), p(b.0, lo, b.1), edge, 2.0, WorldDepth::Tested),
                WorldLine::new(p(a.0, hi, a.1), p(b.0, hi, b.1), edge, 2.0, WorldDepth::Tested),
                WorldLine::new(p(a.0, lo, a.1), p(a.0, hi, a.1), edge, 2.0, WorldDepth::Tested),
            ]
        })
        .collect();
    out.push(WorldLine::new(p(0.5, 1.0, 0.5), p(0.5, 1.0 + BEAM, 0.5), 0x9900_0000 | rgb, 3.0, WorldDepth::ThroughWalls));
    out
}

/// Fades out over the last 3 seconds.
fn alpha(left: i64, full: u32) -> u32 {
    let k = (left.clamp(0, 60) as f64) / 60.0;
    (f64::from(full) * k) as u32
}

pub fn draw(_host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
    if !matches!(event.stage, RenderLevelStage::AfterTranslucentBlocks) {
        return;
    }
    let now = event.view.game_time;
    let (mut quads, mut edges) = (Vec::new(), Vec::new());
    {
        let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
        shown.retain_mut(|m| {
            let until = *m.until.get_or_insert(now + m.left);
            until > now
        });
        for m in shown.iter() {
            let left = m.until.unwrap_or(now) - now;
            quads.extend(beam(m.pos, (alpha(left, 0x60) << 24) | m.colour));
            let a = alpha(left, 0xFF);
            edges.extend(lines(m.pos, m.colour).into_iter().map(|mut l| {
                l.color = ((((l.color >> 24) * a) / 0xFF) << 24) | (l.color & 0xFF_FFFF);
                l
            }));
        }
    }
    if !quads.is_empty()
        && let Err(e) = event.draw.quads(&quads)
    {
        eprintln!("[share] drawing beams: {e:?}");
    }
    if !edges.is_empty()
        && let Err(e) = event.draw.lines(&edges)
    {
        eprintln!("[share] drawing marks: {e:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_beam_has_eight_faces_and_a_mark_thirteen_lines() {
        assert_eq!(beam([0, 0, 0], 0x6000_FFFF).len(), 8);
        assert_eq!(lines([0, 0, 0], 0x00FFFF).len(), 13);
    }

    #[test]
    fn marks_fade_in_their_last_three_seconds() {
        assert_eq!(alpha(600, 0xFF), 0xFF);
        assert_eq!(alpha(0, 0xFF), 0);
        assert!(alpha(30, 0xFF) < 0xFF);
    }
}
