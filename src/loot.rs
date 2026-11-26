//! Loot for each player: a mob killed near friends drops its items once for each player near,
//! each copy only its player may pick up, and its experience goes to each of them.
use crate::rules::{self, Near};
use crate::world;
use mod_api::{Host, ItemDrop, LivingDropsEvent, LivingExperienceDropEvent, PlayerRef, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::sync::Mutex;

/// Experience to give on the next tick: (player, points).
static OWED: Mutex<Vec<(u128, u32)>> = Mutex::new(Vec::new());

fn on(host: &dyn Host) -> bool {
    crate::enabled(host, crate::setting::LOOT)
}

/// The recipients of the loot of `entity`, dying with `killer` to thank.
fn recipients(host: &mut dyn ServerHost, entity: u32, killer: Option<u128>) -> Vec<u128> {
    let read: &dyn Host = &*host;
    let Some(mob) = read.entity(entity) else { return Vec::new() };
    let (Some(at), Some(level)) = (mob.position(read), mob.level(read)) else { return Vec::new() };
    let players: Vec<Near> = host
        .players()
        .server_players()
        .into_iter()
        .filter_map(|p: PlayerRef| {
            let pos = p.entity().position(read)?;
            let playing = p.level(read) == Some(level) && world::playing(read, p);
            Some(Near { uuid: p.uuid(), pos, playing })
        })
        .collect();
    rules::recipients(at, killer, &players)
}


pub fn drops(host: &mut dyn ServerHost, event: &mut LivingDropsEvent<'_>) {
    if !on(&*host) || event.cancelled || event.player.is_some() || !event.recently_hit {
        return;
    }
    let killer = u32::try_from(event.source.causing).ok().and_then(|id| world::by_id(host, id)).map(|p| p.uuid());
    let who = recipients(host, event.entity, killer);
    if !rules::shared(event.kind, event.player.is_some(), event.recently_hit, who.len()) {
        return;
    }
    let made = rules::copies(event.drops.as_slice(), &who, |d: &ItemDrop| d.target, |d, r| {
        let mut copy = d.clone();
        copy.target = Some(r);
        copy
    });
    *event.drops = made;
}

pub fn experience(host: &mut dyn ServerHost, event: &mut LivingExperienceDropEvent<'_>) {
    if !on(&*host) || event.cancelled || event.amount <= 0 || event.kind == "minecraft:player" {
        return;
    }
    let who = recipients(host, event.entity, event.attacking_player);
    if !rules::shared(event.kind, false, true, who.len()) {
        return;
    }
    // Orbs would go to whoever stands nearest, so each player is given the points instead.
    let points = event.amount as u32;
    event.amount = 0;
    OWED.lock().unwrap_or_else(|e| e.into_inner()).extend(who.into_iter().map(|w| (w, points)));
}

/// Gives the experience owed, on the tick after the deaths.
pub fn tick(host: &mut dyn ServerHost) {
    let owed = std::mem::take(&mut *OWED.lock().unwrap_or_else(|e| e.into_inner()));
    if owed.is_empty() {
        return;
    }
    for (uuid, points) in owed {
        if let Some(p) = world::by_uuid(host, uuid) {
            p.give_experience_points(host, points);
        }
    }
}
