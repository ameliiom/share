//! Pets for both, and team hit. One registry of the tamed animals in loaded levels feeds both:
//! - a friend's wolf, cat or parrot sits or stands on the other player's empty-hand click and
//!   follows them while its owner is away (the tamer stays the real owner; tamed horses are
//!   anyone's to ride, as in vanilla). Wolves defending a friend is not in: neither a set target
//!   nor anger made a tamed wolf fight for anyone but its owner (measured 2026-11-26);
//! - a blow on a mob a friend hit within 2 s does 25% more, never on a player or a pet.
use crate::coop::{self, Follow};
use crate::world;
use mod_api::{Compound, EntityInteractEvent, Hand, Host, InteractionResult, LivingDamagePostEvent, LivingDamagePreEvent, ServerHost, Tag};
#[allow(unused_imports)]
use mod_api::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

#[derive(Default)]
struct State {
    /// Tamed animals: entity id → (owner, kind).
    pets: HashMap<u32, (u128, String)>,
    /// Player blows on mobs: mob id → (player id, tick).
    hits: HashMap<i32, Vec<(u32, u64)>>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);
/// The team hit setting, read each tick for the damage hooks (they get no setting reads).
static TEAM_HIT: AtomicBool = AtomicBool::new(true);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    f(STATE.lock().unwrap_or_else(|e| e.into_inner()).get_or_insert_with(State::default))
}

fn on(host: &dyn Host) -> bool {
    crate::enabled(host, crate::setting::PETS)
}

/// Records `id` when it is a tamed animal with an owner.
pub fn seen(host: &mut dyn ServerHost, id: u32, kind: &str) {
    let read: &dyn Host = &*host;
    let Some(living) = read.entity(id).and_then(|e| read.living(e)) else { return };
    if living.is_tamed(read) == Some(true)
        && let Some(owner) = living.owner(read)
    {
        with(|s| s.pets.insert(id, (owner, kind.to_string())));
    }
}

pub fn tamed(id: u32, kind: &str, owner: Option<u128>) {
    if let Some(owner) = owner {
        with(|s| s.pets.insert(id, (owner, kind.to_string())));
    }
}

pub fn gone(id: u32) {
    with(|s| {
        s.pets.remove(&id);
        s.hits.remove(&(id as i32));
    });
}

fn sitting(host: &mut dyn ServerHost, id: u32) -> bool {
    let entity = (&*host as &dyn Host).entity(id);
    entity.and_then(|e| e.nbt(host)).is_some_and(|n| matches!(n.get(&"Sitting".into()), Some(Tag::Byte(b)) if *b != 0))
}

/// A friend's empty-hand click on a pet that isn't theirs: sit or stand, as for its owner.
pub fn interact(host: &mut dyn ServerHost, event: &EntityInteractEvent<'_>) -> Option<InteractionResult> {
    if event.hand != Hand::Main || event.stack.is_some_and(|s| s.count() > 0) || !on(&*host) {
        return None;
    }
    let id = u32::try_from(event.entity).ok()?;
    let owner = with(|s| s.pets.get(&id).filter(|(_, k)| coop::PETS.contains(&k.as_str())).map(|(o, _)| *o))?;
    let clicker = host.players().acting_player()?;
    if clicker.uuid() == owner {
        return None;
    }
    let sit = !sitting(host, id);
    let nbt: Compound = [("Sitting".into(), Tag::Byte(i8::from(sit)))].into_iter().collect();
    let entity = (&*host as &dyn Host).entity(id)?;
    entity.merge_nbt(host, nbt);
    if sit && let Some(mob) = (&*host as &dyn Host).mob(entity) {
        mob.set_target(host, None);
    }
    Some(InteractionResult::SuccessServer)
}

/// A player's blow: remembered for team hit.
pub fn damaged(event: &LivingDamagePostEvent<'_>) {
    let Ok(causing) = u32::try_from(event.source.causing) else { return };
    if world::is_player_id(causing) {
        let now = world::now();
        with(|s| s.hits.entry(event.entity).or_default().push((causing, now)));
    }
}

/// Team hit: more damage when a friend hit the mob just before.
pub fn damage(event: &mut LivingDamagePreEvent<'_>) {
    if !TEAM_HIT.load(Ordering::Relaxed) {
        return;
    }
    let Ok(causing) = u32::try_from(event.source.causing) else { return };
    if !world::is_player_id(causing) {
        return;
    }
    let pet = u32::try_from(event.entity).is_ok_and(|id| with(|s| s.pets.contains_key(&id)));
    let hits = with(|s| s.hits.get(&event.entity).cloned().unwrap_or_default());
    let before = event.container.new_damage();
    let after = coop::team_damage(before, causing, &hits, world::now(), event.kind == "minecraft:player" || pet);
    if after != before {
        event.container.set_new_damage(after);
    }
}

/// Whether the pet is going for something (then it doesn't follow).
fn hunting(host: &dyn Host, e: mod_api::EntityRef) -> bool {
    host.mob(e).is_some_and(|m| m.target(host).is_some())
}

/// Each tick: team hit's setting and old blows; every half second, pets follow a friend while
/// their owner is away.
pub fn tick(host: &mut dyn ServerHost) {
    let now = world::now();
    TEAM_HIT.store(crate::enabled(&*host, crate::setting::TEAM_HIT), Ordering::Relaxed);
    with(|s| {
        s.hits.retain(|_, v| {
            v.retain(|(_, at)| now.saturating_sub(*at) <= coop::TEAM_TICKS);
            !v.is_empty()
        })
    });
    if !on(&*host) || !now.is_multiple_of(10) {
        return;
    }
    let pets: Vec<(u32, u128)> = with(|s| s.pets.iter().filter(|(_, (_, k))| coop::PETS.contains(&k.as_str())).map(|(id, (o, _))| (*id, *o)).collect());
    let players = host.players().server_players();
    for (id, owner) in pets {
        let read: &dyn Host = &*host;
        let Some(e) = read.entity(id) else { continue };
        let (Some(at), level) = (e.position(read), e.level(read)) else { continue };
        let owner_distance = players
            .iter()
            .find(|p| p.uuid() == owner && p.level(read) == level)
            .and_then(|p| p.entity().position(read))
            .map(|q| world::distance(q, at));
        let friend = players
            .iter()
            .filter(|p| p.uuid() != owner && p.level(read) == level && world::playing(read, **p))
            .filter_map(|p| p.entity().position(read).map(|q| (world::distance(q, at), q)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((d, to)) = friend else { continue };
        if owner_distance.is_some_and(|o| o <= 48.0) || hunting(read, e) {
            continue;
        }
        let sitting = sitting(host, id);
        match coop::follow(sitting, owner_distance, d) {
            Follow::Stay => {}
            Follow::Walk => {
                if let Some(mob) = (&*host as &dyn Host).mob(e)
                    && let Err(e) = mob.set_wanted_position(host, to, 1.2)
                {
                    eprintln!("[share] a pet walking: {e:?}");
                }
            }
            Follow::Teleport => {
                e.teleport_to(host, [to[0] + 1.0, to[1], to[2] + 1.0], None);
            }
        }
    }
}
