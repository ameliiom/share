//! Share: co-op comforts for players together. See `docs/design.md`.
//!
//! - **The rules** (`rules.rs`, `coop.rs`): every decision on plain values, unit tested.
//! - **World** (`world.rs`): the clock and the players, read one way for all parts.
//! - **Loot** (`loot.rs`): a mob's drops and experience once for each player near.
//! - **Marks** (`marks.rs`, `client.rs`): J marks a spot with a beam and your name for 30 s.
//! - **Revive** (`revive.rs`): down for 30 s instead of dead while a friend can come.
//! - **Throw** (`throw.rs`): Q while looking at a friend hands them the item.
//! - **Pets** (`pets.rs`): a friend's pets obey you too; team hit on a mob a friend just hit.
mod client;
pub mod coop;
mod loot;
mod marks;
mod payloads;
mod pets;
mod revive;
pub mod rules;
mod throw;
mod world;

use mod_api::{
    AttackEntityEvent, ClientHost, ClientTickPreEvent, EntityInteractEvent, EntityJoinLevelEvent, EntityLeaveLevelEvent, EntityTameEvent, Flow,
    Hooks, Host, HostSide, IPayloadContext, InteractionResult, ItemTossEvent, KeyRegistration, LevelRandomHost, LivingDamagePostEvent,
    LivingDamagePreEvent, LivingDeathEvent, LivingDropsEvent, LivingExperienceDropEvent, PayloadId, PlayerLoggedOutEvent, PlayerRef, Registrar,
    RegistrationContext, RenderLevelStageEvent, ServerHost, ServerTickEvent, ServerTickPhase, SettingSpec,
};
#[allow(unused_imports)]
use mod_api::prelude::*;

pub(crate) const MOD_ID: &str = "share";
/// The mark key: J, free in vanilla and the other mods (B emotes, Y a mod screen, Z and U Builder).
const MARK_KEY: &str = "mark";

/// Settings (Options > Mods > Share), each on by default.
pub(crate) mod setting {
    pub const LOOT: &str = "loot";
    pub const MARKS: &str = "marks";
    pub const REVIVE: &str = "revive";
    pub const THROW: &str = "throw";
    pub const PETS: &str = "pets";
    pub const TEAM_HIT: &str = "team_hit";
    /// Every setting, each a toggle on by default, labelled `share.config.<key>`.
    pub const ALL: [&str; 6] = [LOOT, MARKS, REVIVE, THROW, PETS, TEAM_HIT];
}

/// Whether the setting `key` is on (a toggle, on unless set to false).
pub(crate) fn enabled(host: &dyn Host, key: &str) -> bool {
    host.setting(MOD_ID, key).is_none_or(|v| v.trim() != "false")
}

fn on_press(press: payloads::Press, cx: &mut dyn IPayloadContext) -> Result<(), String> {
    let p: PlayerRef = cx.player().map_err(|e| format!("{e:?}"))?;
    let host = cx.server_host().map_err(|e| format!("{e:?}"))?;
    if let [x, y, z] = press.at.as_slice() {
        marks::press(host, p, [x.0, y.0, z.0]);
    }
    Ok(())
}

fn on_marks(view: payloads::Marks, _: &mut dyn IPayloadContext) -> Result<(), String> {
    client::received(view);
    Ok(())
}

#[derive(Default)]
pub struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let mut key = KeyRegistration::new(MARK_KEY, "key.keyboard.j");
        key.label = "key.share.mark".into();
        r.register_key_mapping(key);
        for key in setting::ALL {
            if let Err(e) = r.register_setting(SettingSpec::toggle(key, format!("share.config.{key}"), true)) {
                eprintln!("[share] setting {key}: {e:?}");
            }
        }
        let mut handlers = r.payload_handlers();
        let mut v1 = handlers.registrar("1").optional();
        let _ = payloads::PRESS.set(v1.play_to_server_typed(PayloadId("share:press".into()), on_press));
        let _ = payloads::MARKS.set(v1.play_to_client_typed(PayloadId("share:marks".into()), on_marks));
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _: &mut ClientTickPreEvent) {
        let mut pressed = false;
        while host.consume_click(MOD_ID, MARK_KEY) {
            pressed = true;
        }
        if pressed {
            client::press(host);
        }
    }

    fn render_level_stage(&self, host: &dyn ClientHost, event: &mut RenderLevelStageEvent<'_>) {
        client::draw(host, event);
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
        marks::joined(host, event.entity, event.kind);
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
            marks::tick(host);
            revive::tick(host);
            pets::tick(host);
        }
    }

    fn player_logged_out(&self, _: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        marks::forget(event.player);
        revive::forget(event.player);
    }
}
