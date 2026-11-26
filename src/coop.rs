//! The pure rules of the co-op comforts (revive, throw, pets, team hit), on plain values, unit
//! tested: what the server code asks before it acts.

/// A downed player waits this long for a friend before dying.
pub const DOWN_TICKS: u64 = 600;
/// A friend sneaking next to a downed player this long revives them.
pub const REVIVE_TICKS: u32 = 60;
/// How near (blocks) a reviving friend must be.
pub const REVIVE_REACH: f64 = 2.0;
/// Health a revived player gets back.
pub const REVIVED_HEALTH: f32 = 4.0;
/// How far an item can be thrown to a friend.
pub const THROW_REACH: f64 = 16.0;
/// How far a friend's body may be off the line of sight to count as aimed at.
pub const THROW_AIM: f64 = 0.9;
/// A friend's hit counts for team hit this long.
pub const TEAM_TICKS: u64 = 40;
/// Team hit's extra damage.
pub const TEAM_BONUS: f32 = 1.25;

/// Damage types no one is saved from: `/kill` and the void.
const FINAL: [&str; 2] = ["minecraft:generic_kill", "minecraft:out_of_world"];

/// Whether a player whose health ran out goes down instead of dying: in survival or adventure,
/// not already down, not killed by `/kill` or the void, with another playing player online.
pub fn goes_down(damage_type: &str, playing: bool, already_down: bool, friends_online: usize) -> bool {
    playing && !already_down && friends_online > 0 && !FINAL.contains(&damage_type)
}

/// What happens to a downed player this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Down {
    /// Still waiting; `progress` ticks of reviving so far.
    Waiting { progress: u32 },
    Revived,
    /// Time is up: they die.
    Dies,
}

/// One tick of a downed player: a friend sneaking near adds to the revive, none resets it;
/// a revive completes before the time runs out.
pub fn down_tick(progress: u32, reviving: bool, now: u64, until: u64) -> Down {
    let progress = if reviving { progress + 1 } else { 0 };
    if progress >= REVIVE_TICKS {
        Down::Revived
    } else if now >= until {
        Down::Dies
    } else {
        Down::Waiting { progress }
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The friend an item thrown from `eye` along `view` (unit length) is aimed at: the nearest one
/// whose body centre lies within [`THROW_REACH`] ahead and [`THROW_AIM`] of the line. Gives
/// the index into `friends`.
pub fn aimed(eye: [f64; 3], view: [f64; 3], friends: &[[f64; 3]]) -> Option<usize> {
    friends
        .iter()
        .enumerate()
        .filter_map(|(i, body)| {
            let to = sub(*body, eye);
            let along = dot(to, view);
            let off = (dot(to, to) - along * along).max(0.0).sqrt();
            (along > 0.5 && along <= THROW_REACH && off <= THROW_AIM).then_some((along, i))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, i)| i)
}

/// Team hit: the damage of `attacker`'s blow on a mob that the players in `hits` (player, tick)
/// struck, at `now`. A friend's hit within [`TEAM_TICKS`] adds [`TEAM_BONUS`]; never on a
/// player or a tamed pet.
pub fn team_damage(damage: f32, attacker: u32, hits: &[(u32, u64)], now: u64, victim_is_player_or_pet: bool) -> f32 {
    let friend_hit = hits.iter().any(|(who, at)| *who != attacker && now.saturating_sub(*at) <= TEAM_TICKS);
    if victim_is_player_or_pet || !friend_hit { damage } else { damage * TEAM_BONUS }
}

/// Kinds of pets a friend may order to sit and that follow a friend (horses can be ridden by
/// anyone once tamed, as in vanilla).
pub const PETS: [&str; 3] = ["minecraft:wolf", "minecraft:cat", "minecraft:parrot"];

/// What a pet does for a friend while its owner is away (offline, elsewhere or 48+ blocks off).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    Stay,
    /// Walk to the friend.
    Walk,
    /// Too far behind: appear beside the friend, as a pet does for its owner.
    Teleport,
}

/// A pet's move toward `friend_distance` blocks off, when its owner is `owner_distance` away
/// (None: offline or in another dimension). Sitting pets stay.
pub fn follow(sitting: bool, owner_distance: Option<f64>, friend_distance: f64) -> Follow {
    let owner_away = owner_distance.is_none_or(|d| d > 48.0);
    if sitting || !owner_away || friend_distance > 48.0 {
        Follow::Stay
    } else if friend_distance > 12.0 {
        Follow::Teleport
    } else if friend_distance > 4.0 {
        Follow::Walk
    } else {
        Follow::Stay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_goes_down_only_with_a_friend_online() {
        assert!(goes_down("minecraft:mob_attack", true, false, 1));
        assert!(!goes_down("minecraft:mob_attack", true, false, 0));
    }

    #[test]
    fn kill_the_void_creative_and_a_second_death_are_final() {
        assert!(!goes_down("minecraft:generic_kill", true, false, 1));
        assert!(!goes_down("minecraft:out_of_world", true, false, 1));
        assert!(!goes_down("minecraft:mob_attack", false, false, 1));
        assert!(!goes_down("minecraft:mob_attack", true, true, 1));
    }

    #[test]
    fn three_seconds_of_sneaking_revive() {
        let mut p = 0;
        for t in 0..REVIVE_TICKS - 1 {
            match down_tick(p, true, t as u64, DOWN_TICKS) {
                Down::Waiting { progress } => p = progress,
                other => panic!("{other:?} at {t}"),
            }
        }
        assert_eq!(down_tick(p, true, 59, DOWN_TICKS), Down::Revived);
    }

    #[test]
    fn stepping_away_resets_the_revive() {
        assert_eq!(down_tick(50, false, 10, DOWN_TICKS), Down::Waiting { progress: 0 });
    }

    #[test]
    fn thirty_seconds_alone_is_death() {
        assert_eq!(down_tick(0, false, DOWN_TICKS, DOWN_TICKS), Down::Dies);
        assert_eq!(down_tick(REVIVE_TICKS - 1, true, DOWN_TICKS, DOWN_TICKS), Down::Revived);
    }

    #[test]
    fn a_throw_goes_to_the_friend_looked_at() {
        let eye = [0.0, 1.6, 0.0];
        let north = [0.0, 0.0, -1.0];
        let friends = [[5.0, 1.0, -5.0], [0.3, 1.0, -8.0], [0.0, 1.0, -4.0]];
        assert_eq!(aimed(eye, north, &friends), Some(2));
    }

    #[test]
    fn a_throw_past_reach_behind_or_aside_goes_to_no_one() {
        let eye = [0.0, 1.6, 0.0];
        let north = [0.0, 0.0, -1.0];
        assert_eq!(aimed(eye, north, &[[0.0, 1.0, -20.0]]), None);
        assert_eq!(aimed(eye, north, &[[0.0, 1.0, 5.0]]), None);
        assert_eq!(aimed(eye, north, &[[3.0, 1.0, -5.0]]), None);
    }

    #[test]
    fn team_hit_needs_a_friends_recent_hit() {
        assert_eq!(team_damage(8.0, 1, &[(2, 100)], 130, false), 10.0);
        assert_eq!(team_damage(8.0, 1, &[(2, 100)], 141, false), 8.0);
        assert_eq!(team_damage(8.0, 1, &[(1, 100)], 101, false), 8.0);
    }

    #[test]
    fn team_hit_never_on_players_or_pets() {
        assert_eq!(team_damage(8.0, 1, &[(2, 100)], 101, true), 8.0);
    }

    #[test]
    fn pets_follow_a_friend_only_while_the_owner_is_away() {
        assert_eq!(follow(false, Some(5.0), 20.0), Follow::Stay);
        assert_eq!(follow(false, None, 20.0), Follow::Teleport);
        assert_eq!(follow(false, Some(60.0), 8.0), Follow::Walk);
        assert_eq!(follow(false, None, 2.0), Follow::Stay);
        assert_eq!(follow(true, None, 20.0), Follow::Stay);
        assert_eq!(follow(false, None, 100.0), Follow::Stay);
    }
}
