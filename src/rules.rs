//! The pure rules, on plain values: who gets a copy of a mob's loot, and how marks are kept.
//! No game calls here, so every rule is unit tested.

/// A block position.
pub type Pos = [i32; 3];

/// Players within this many blocks of a dying mob get their own copy of its loot.
pub const NEAR: f64 = 32.0;
/// How far a mark can be placed.
pub const REACH: f32 = 128.0;
/// A mark lasts 30 seconds.
pub const MARK_TICKS: u64 = 600;
/// At most this many marks per player; a new one past it replaces that player's oldest.
pub const MARKS_EACH: usize = 5;

/// A player as the loot rule sees them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Near {
    pub uuid: u128,
    pub pos: [f64; 3],
    /// In survival or adventure and in the mob's level (spectators and creative get no copy).
    pub playing: bool,
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// Who gets a copy of the loot of a mob that died at `at`: every playing player within
/// [`NEAR`], and the `killer` wherever they stand if they play. Each once, killer first.
/// Fewer than two means vanilla (solo play is untouched).
pub fn recipients(at: [f64; 3], killer: Option<u128>, players: &[Near]) -> Vec<u128> {
    let mut out: Vec<u128> = Vec::new();
    if let Some(k) = killer
        && players.iter().any(|p| p.uuid == k && p.playing)
    {
        out.push(k);
    }
    for p in players {
        if p.playing && distance(p.pos, at) <= NEAR && !out.contains(&p.uuid) {
            out.push(p.uuid);
        }
    }
    out
}

/// Whether the loot of a death is shared: a mob (never a player, whose inventory it is, nor an
/// armor stand) that a player hurt within the last 100 ticks, with two players or more to share.
pub fn shared(kind: &str, is_player: bool, recently_hit: bool, recipients: usize) -> bool {
    !is_player && kind != "minecraft:player" && kind != "minecraft:armor_stand" && recently_hit && recipients >= 2
}

/// One copy of each drop for each recipient, its owner set: `owned(drop, recipient)` makes the
/// copy. Drops that already have an owner stay as they are, once.
pub fn copies<D: Clone>(drops: &[D], recipients: &[u128], owner: impl Fn(&D) -> Option<u128>, owned: impl Fn(&D, u128) -> D) -> Vec<D> {
    let mut out = Vec::with_capacity(drops.len() * recipients.len());
    for d in drops {
        if owner(d).is_some() {
            out.push(d.clone());
            continue;
        }
        for r in recipients {
            out.push(owned(d, *r));
        }
    }
    out
}

/// A mark placed by a player.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub owner: u128,
    pub name: String,
    pub dimension: String,
    pub pos: Pos,
    /// The server tick it goes at.
    pub until: u64,
    /// The text display showing the name, once summoned.
    pub label: Option<u32>,
}

/// What pressing the mark key did.
#[derive(Clone, Debug, PartialEq)]
pub enum Pressed {
    /// A new mark; `replaced` is the owner's oldest, taken out to stay within [`MARKS_EACH`].
    Placed { replaced: Option<Mark> },
    /// The player's own mark there was taken back.
    Removed(Mark),
}

/// The marks of every player.
#[derive(Default, Debug)]
pub struct Marks {
    pub list: Vec<Mark>,
}

/// Near enough to count as the same spot (the block or one beside it).
fn same_spot(a: Pos, b: Pos) -> bool {
    (0..3).all(|i| (a[i] - b[i]).abs() <= 1)
}

impl Marks {
    /// A player pressed the mark key looking at `pos`: their own mark there goes, else a new one
    /// lasting [`MARK_TICKS`] from `now`, the oldest of theirs going past [`MARKS_EACH`].
    pub fn press(&mut self, owner: u128, name: &str, dimension: &str, pos: Pos, now: u64) -> Pressed {
        if let Some(i) = self.list.iter().position(|m| m.owner == owner && m.dimension == dimension && same_spot(m.pos, pos)) {
            return Pressed::Removed(self.list.remove(i));
        }
        let mut replaced = None;
        if self.list.iter().filter(|m| m.owner == owner).count() >= MARKS_EACH
            && let Some(i) = self.list.iter().position(|m| m.owner == owner)
        {
            replaced = Some(self.list.remove(i));
        }
        self.list.push(Mark { owner, name: name.into(), dimension: dimension.into(), pos, until: now + MARK_TICKS, label: None });
        Pressed::Placed { replaced }
    }

    /// Takes out and returns the marks whose time is up at `now`.
    pub fn expire(&mut self, now: u64) -> Vec<Mark> {
        let (gone, kept) = std::mem::take(&mut self.list).into_iter().partition(|m| m.until <= now);
        self.list = kept;
        gone
    }
}

/// A player's colour (0xRRGGBB), the same for the beam and the name, from their UUID.
pub fn colour(uuid: u128) -> u32 {
    const PALETTE: [u32; 8] = [0x55FFFF, 0xFFAA00, 0x55FF55, 0xFF55FF, 0xFFFF55, 0xFF5555, 0x5599FF, 0xFFFFFF];
    let folded = (uuid as u64) ^ ((uuid >> 64) as u64);
    PALETTE[(folded % PALETTE.len() as u64) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: u128 = 1;
    const B: u128 = 2;
    const C: u128 = 3;

    fn near(uuid: u128, x: f64, playing: bool) -> Near {
        Near { uuid, pos: [x, 64.0, 0.0], playing }
    }

    #[test]
    fn solo_play_stays_vanilla() {
        let r = recipients([0.0, 64.0, 0.0], Some(A), &[near(A, 3.0, true)]);
        assert_eq!(r, vec![A]);
        assert!(!shared("minecraft:zombie", false, true, r.len()));
    }

    #[test]
    fn the_killer_is_counted_once_and_first() {
        let r = recipients([0.0, 64.0, 0.0], Some(B), &[near(A, 3.0, true), near(B, 2.0, true)]);
        assert_eq!(r, vec![B, A]);
    }

    #[test]
    fn far_spectating_and_creative_players_get_none() {
        let players = [near(A, 1.0, true), near(B, 40.0, true), near(C, 2.0, false)];
        assert_eq!(recipients([0.0, 64.0, 0.0], Some(A), &players), vec![A]);
    }

    #[test]
    fn a_far_killer_still_gets_theirs() {
        let players = [near(A, 60.0, true), near(B, 5.0, true)];
        assert_eq!(recipients([0.0, 64.0, 0.0], Some(A), &players), vec![A, B]);
    }

    #[test]
    fn a_spectating_killer_gets_none() {
        let players = [near(A, 1.0, false), near(B, 5.0, true)];
        assert_eq!(recipients([0.0, 64.0, 0.0], Some(A), &players), vec![B]);
    }

    #[test]
    fn players_armor_stands_and_unhit_deaths_are_not_shared() {
        assert!(!shared("minecraft:player", true, true, 2));
        assert!(!shared("minecraft:armor_stand", false, true, 2));
        assert!(!shared("minecraft:zombie", false, false, 2));
        assert!(shared("minecraft:zombie", false, true, 2));
    }

    #[test]
    fn each_recipient_gets_one_copy_of_each_drop() {
        // (item, owner)
        let drops = [("bone", None), ("arrow", None), ("theirs", Some(C))];
        let out = copies(&drops, &[A, B], |d| d.1, |d, r| (d.0, Some(r)));
        assert_eq!(out, vec![("bone", Some(A)), ("bone", Some(B)), ("arrow", Some(A)), ("arrow", Some(B)), ("theirs", Some(C))]);
        for who in [A, B] {
            assert_eq!(out.iter().filter(|d| d.1 == Some(who) && d.0 == "bone").count(), 1);
        }
    }

    #[test]
    fn pressing_on_your_own_mark_removes_it() {
        let mut m = Marks::default();
        assert_eq!(m.press(A, "a", "o", [0, 0, 0], 0), Pressed::Placed { replaced: None });
        assert!(matches!(m.press(A, "a", "o", [1, 0, 0], 5), Pressed::Removed(_)));
        assert!(m.list.is_empty());
    }

    #[test]
    fn a_friends_mark_is_not_yours_to_remove() {
        let mut m = Marks::default();
        m.press(A, "a", "o", [0, 0, 0], 0);
        assert_eq!(m.press(B, "b", "o", [0, 0, 0], 0), Pressed::Placed { replaced: None });
        assert_eq!(m.list.len(), 2);
    }

    #[test]
    fn the_sixth_mark_replaces_the_oldest() {
        let mut m = Marks::default();
        m.press(B, "b", "o", [0, 0, 0], 0);
        for i in 0..5 {
            m.press(A, "a", "o", [i * 10, 0, 0], i as u64);
        }
        let Pressed::Placed { replaced: Some(old) } = m.press(A, "a", "o", [100, 0, 0], 9) else { panic!() };
        assert_eq!(old.pos, [0, 0, 0]);
        assert_eq!(old.owner, A);
        assert_eq!(m.list.iter().filter(|x| x.owner == A).count(), MARKS_EACH);
        assert_eq!(m.list.iter().filter(|x| x.owner == B).count(), 1);
    }

    #[test]
    fn marks_go_after_thirty_seconds() {
        let mut m = Marks::default();
        m.press(A, "a", "o", [0, 0, 0], 0);
        m.press(A, "a", "o", [50, 0, 0], 100);
        assert!(m.expire(599).is_empty());
        assert_eq!(m.expire(600).len(), 1);
        assert_eq!(m.list.len(), 1);
    }

    #[test]
    fn same_spot_in_another_dimension_is_a_new_mark() {
        let mut m = Marks::default();
        m.press(A, "a", "overworld", [0, 0, 0], 0);
        assert!(matches!(m.press(A, "a", "nether", [0, 0, 0], 0), Pressed::Placed { .. }));
    }

    #[test]
    fn colours_are_stable() {
        assert_eq!(colour(42), colour(42));
    }
}
