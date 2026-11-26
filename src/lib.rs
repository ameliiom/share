//! Share: co-op comforts for players together. See `docs/design.md`.
//!
//! - **The rules** (`rules.rs` for loot, `coop.rs` for the rest): every decision on plain values, unit tested.
//! - **World** (`world.rs`): the clock and the players, read one way for all parts.
//! - **Loot** (`loot.rs`): a mob's drops and experience once for each player near.
//! - **Revive** (`revive.rs`): down for 30 s instead of dead while a friend can come.
//! - **Throw** (`throw.rs`): Q while looking at a friend hands them the item.
//! - **Pets** (`pets.rs`): a friend's pets obey you too; team hit on a mob a friend just hit.
// Kit (ameliiom/kit), the shared code: the installed Kit mod, or its source in place for
// local builds (`kit_local`, horizon-mods/.cargo/config.toml).
#[cfg(kit_local)]
#[allow(dead_code)]
#[path = "../../kit/src/lib.rs"]
mod kit;
#[cfg(not(kit_local))]
use ::kit;
pub mod coop;
mod loot;
mod pets;
mod revive;
pub mod rules;
mod throw;
mod world;

use mod_api::{
    AttackEntityEvent, EntityInteractEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, EntityTameEvent, Flow,
    Hooks, Host, HostSide, InteractionResult, ItemTossEvent, LevelRandomHost, LivingDamagePostEvent,
    LivingDamagePreEvent, LivingDeathEvent, LivingDropsEvent, LivingExperienceDropEvent, PlayerLoggedOutEvent, Registrar,
    RegistrationContext, ServerHost, ServerTickEvent, ServerTickPhase,
};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub(crate) const MOD_ID: &str = "share";
/// Settings (Options > Mods > Share), each on by default.
pub(crate) mod setting {
    use crate::kit::setting::Toggle;
    pub const LOOT: Toggle = Toggle::on("loot");
    pub const REVIVE: Toggle = Toggle::on("revive");
    pub const THROW: Toggle = Toggle::on("throw");
    pub const PETS: Toggle = Toggle::on("pets");
    pub const TEAM_HIT: Toggle = Toggle::on("team_hit");
    /// Every setting, labelled `share.config.<key>`.
    pub const ALL: [Toggle; 5] = [LOOT, REVIVE, THROW, PETS, TEAM_HIT];
}

/// Whether the setting `key` is on.
pub(crate) fn enabled(host: &dyn Host, key: kit::setting::Toggle) -> bool {
    key.read(host, MOD_ID)
}

/// The name labels of the marks of 0.1–0.2.0 (J), retired for Team's pings: one left in a saved
/// world (a server stopped within a mark's 30 s) is removed when it loads.
const OLD_MARK_TAG: &str = "share.mark";

fn retire_mark_label(host: &mut dyn ServerHost, id: u32, kind: &str) {
    if kind != "minecraft:text_display" {
        return;
    }
    let read: &dyn Host = &*host;
    if let Some(e) = read.entity(id)
        && e.tags(read).is_some_and(|t| t.iter().any(|t| t == OLD_MARK_TAG))
    {
        e.discard(host);
    }
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        for key in setting::ALL {
            if let Err(e) = r.register_setting(key.spec(MOD_ID)) {
                eprintln!("[share] setting {}: {e:?}", key.key);
            }
        }
    }

    /// A dying mob's items: one copy for each player near.
    fn living_drops(&self, host: &mut dyn ServerHost, event: &mut LivingDropsEvent<'_>) -> Flow {
        loot::drops(host, event);
        Flow::Continue
    }

    /// A dying mob's experience: the same points for each player near.
    fn living_experience_drop(&self, host: &mut dyn ServerHost, event: &mut LivingExperienceDropEvent<'_>) -> Flow {
        loot::experience(host, event);
        Flow::Continue
    }

    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        retire_mark_label(host, event.entity, event.kind);
        pets::seen(host, event.entity, event.kind);
        Flow::Continue
    }

    fn entity_leave_level(&self, _: &mut dyn ServerHost, event: &mut EntityLeaveLevelEvent<'_>) {
        pets::gone(event.entity);
    }

    fn entity_tame(&self, _: &mut dyn ServerHost, event: &mut EntityTameEvent<'_>) {
        pets::tamed(event.entity, event.kind, event.owner);
    }

    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        match host {
            HostSide::Server(host) => pets::interact(host, event),
            HostSide::Client(_) => None,
        }
    }

    /// A player's health ran out: down instead, while a friend can come.
    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if revive::death(host, event) { Flow::Cancel } else { Flow::Continue }
    }

    /// A downed player's blows do nothing.
    fn attack_entity(&self, host: &mut dyn ServerHost, _: &mut AttackEntityEvent<'_>) -> Flow {
        if host.players().acting_player().is_some_and(|p| revive::is_down(p.uuid())) { Flow::Cancel } else { Flow::Continue }
    }

    /// Q while looking at a friend hands them the item.
    fn item_toss(&self, host: &mut dyn ServerHost, event: &mut ItemTossEvent) -> Flow {
        throw::tossed(host, event);
        Flow::Continue
    }

    fn living_damage_pre(&self, _: &mut dyn LevelRandomHost, event: &mut LivingDamagePreEvent<'_>) {
        pets::damage(event);
    }

    fn living_damage_post(&self, _: &mut dyn LevelRandomHost, event: &mut LivingDamagePostEvent<'_>) {
        pets::damaged(event);
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            world::tick(host);
            loot::tick(host);
            revive::tick(host);
            pets::tick(host);
        }
    }

    fn player_logged_out(&self, _: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        revive::forget(event.player);
    }
}
