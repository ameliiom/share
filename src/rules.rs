//! The pure loot rules, on plain values: who gets a copy of a mob's loot, and how many.
//! No game calls here, so every rule is unit tested.

/// Players within this many blocks of a dying mob get their own copy of its loot.
pub const NEAR: f64 = 32.0;
/// A player as the loot rule sees them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Near {
    pub uuid: u128,
    pub pos: [f64; 3],
    /// In survival or adventure and in the mob's level (spectators and creative get no copy).
    pub playing: bool,
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
        if p.playing && crate::kit::near::within(p.pos, at, NEAR) && !out.contains(&p.uuid) {
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
}
