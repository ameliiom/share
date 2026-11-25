//! What crosses the network. Registered (version "1", optional) in `register`.
use mod_api::{PayloadCodec, ToClient, ToServer, VarInt};
use std::sync::OnceLock;

/// Client → server: the mark key was pressed looking at the block `at` (x, y, z).
#[derive(PayloadCodec, Clone, Debug)]
pub struct Press {
    pub at: Vec<VarInt>,
}

/// Server → client: every mark in the player's dimension, flat: `at` holds x, y, z of each,
/// `colour` its 0xRRGGBB and `left` its ticks left, in the same order.
#[derive(PayloadCodec, Clone, Debug, Default, PartialEq)]
pub struct Marks {
    pub at: Vec<VarInt>,
    pub colour: Vec<VarInt>,
    pub left: Vec<VarInt>,
}

pub static PRESS: OnceLock<ToServer<Press>> = OnceLock::new();
pub static MARKS: OnceLock<ToClient<Marks>> = OnceLock::new();
