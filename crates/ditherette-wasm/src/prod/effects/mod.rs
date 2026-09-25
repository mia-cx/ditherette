//! Production ordered effects, copied from the frozen `spec::effects` reference.
//!
//! Exact modes must match the reference byte-for-byte. See `README.md` for
//! optimizations and their evidence.

pub mod chain;
pub mod channel;
pub mod image;
pub mod levels;
pub mod operation;
pub mod recipe;
pub mod table;

pub use chain::{apply_chain, ChannelFn, Effect, EffectContext, Needs, Step};
pub use image::EffectImage;
pub use operation::{apply_effects, apply_in_place, EffectsRequest};
pub use recipe::{decode_effects, BuiltinEffect, EffectStep};
