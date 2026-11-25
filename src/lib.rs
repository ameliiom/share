//! Share: co-op comforts. A mob killed near friends drops its loot and experience once for each
//! player near, each copy only its player may pick up; J marks a spot for your friends with a
//! light beam and your name for 30 seconds. See `docs/design.md`.
//!
//! - **The rules** (`rules.rs`): who gets a copy, how marks are kept, on plain values.
//! - **Loot** (`loot.rs`): the server's copies of drops and experience.
//! - **Marks** (`marks.rs`): the server's marks, names and what each player is sent.
//! - **The client** (`client.rs`): the key and the beams.
mod client;
mod loot;
mod marks;
mod payloads;
pub mod rules;

use mod_api::{
    ClientHost, ClientTickPreEvent, EntityJoinLevelEvent, Flow, Hooks, IPayloadContext, KeyRegistration, LivingDropsEvent, LivingExperienceDropEvent,
    PayloadId, PlayerLoggedOutEvent, PlayerRef, Registrar, RegistrationContext, RenderLevelStageEvent, ServerHost, ServerTickEvent, ServerTickPhase,
    SettingSpec,
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
        for (key, label) in [(setting::LOOT, "share.config.loot"), (setting::MARKS, "share.config.marks")] {
            if let Err(e) = r.register_setting(SettingSpec::toggle(key, label, true)) {
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
        Flow::Continue
    }

    fn server_tick(&self, host: &mut dyn ServerHost, event: &mut ServerTickEvent<'_>) {
        if matches!(event.phase, ServerTickPhase::ServerTickEnd { .. }) {
            loot::tick(host);
            marks::tick(host);
        }
    }

    fn player_logged_out(&self, _: &mut dyn ServerHost, event: &mut PlayerLoggedOutEvent<'_>) {
        marks::forget(event.player);
    }
}
