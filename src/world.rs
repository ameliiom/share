//! What every server part reads the same way: the clock, who plays, players by UUID or entity id.
use mod_api::{GameMode, Host, PlayerRef, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

/// The world's game time at the last server tick (one clock for every dimension).
static NOW: AtomicU64 = AtomicU64::new(0);
/// The entity ids of the online players, refreshed each tick (hooks without a host read it).
static PLAYER_IDS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

pub fn now() -> u64 {
    NOW.load(Ordering::Relaxed)
}

pub fn is_player_id(id: u32) -> bool {
    id != 0 && PLAYER_IDS.lock().unwrap_or_else(|e| e.into_inner()).contains(&id)
}

/// Reads the clock and the players at the end of a server tick.
pub fn tick(host: &mut dyn ServerHost) {
    let players = host.players().server_players();
    let read: &dyn Host = &*host;
    if let Some(t) = players.first().and_then(|p| p.level(read)).and_then(|l| l.game_time(read)).and_then(|t| u64::try_from(t).ok()) {
        NOW.store(t, Ordering::Relaxed);
    }
    *PLAYER_IDS.lock().unwrap_or_else(|e| e.into_inner()) = players.iter().map(|p| p.entity().id()).collect();
}

/// In survival or adventure (spectators and creative players take no part).
pub fn playing(host: &dyn Host, p: PlayerRef) -> bool {
    matches!(p.game_mode(host), Some(GameMode::Survival | GameMode::Adventure))
}

pub fn by_uuid(host: &mut dyn ServerHost, uuid: u128) -> Option<PlayerRef> {
    host.players().server_players().into_iter().find(|p| p.uuid() == uuid)
}

pub fn by_id(host: &mut dyn ServerHost, id: u32) -> Option<PlayerRef> {
    if id == 0 {
        return None;
    }
    host.players().server_players().into_iter().find(|p| p.entity().id() == id)
}

pub fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}
