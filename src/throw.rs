//! Throw to a friend: an item dropped (Q) while looking at a friend within 16 blocks appears at
//! the friend's feet, theirs alone to pick up, so it lands in their inventory at once (or waits
//! there when it is full).
use crate::coop;
use crate::world;
use mod_api::{Host, ItemTossEvent, ServerHost};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub fn tossed(host: &mut dyn ServerHost, event: &mut ItemTossEvent) {
    if !crate::enabled(&*host, crate::setting::THROW) {
        return;
    }
    let Some(p) = world::by_uuid(host, event.player) else { return };
    let players = host.players().server_players();
    let read: &dyn Host = &*host;
    let (Some(eye), Some(view), level) = (p.entity().eye_position(read), p.entity().view_vector(read), p.level(read)) else { return };
    let friends: Vec<_> = players
        .into_iter()
        .filter(|f| f.uuid() != p.uuid() && f.level(read) == level && world::playing(read, *f) && !crate::revive::is_down(f.uuid()))
        .filter_map(|f| f.entity().position(read).map(|pos| (f, pos)))
        .collect();
    let bodies: Vec<[f64; 3]> = friends.iter().map(|(_, pos)| [pos[0], pos[1] + 0.9, pos[2]]).collect();
    let Some(i) = coop::aimed(eye, view, &bodies) else { return };
    let (friend, feet) = friends[i];
    let item = event.item;
    item.set_target(host, Some(friend.uuid()));
    item.set_pickup_delay(host, 0);
    item.entity().teleport_to(host, [feet[0], feet[1] + 0.1, feet[2]], None);
    item.entity().set_velocity(host, [0.0, 0.0, 0.0]);
}
