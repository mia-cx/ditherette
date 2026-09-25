//! Ordered colour effects that run before palette output.
//!
//! See `spec.md` for the carrier, ordering, placement, and plugin boundary.

pub mod chain;
pub mod channel;
pub mod image;
pub mod levels;
pub mod operation;
pub mod recipe;

pub use chain::{apply_chain, Effect, EffectContext, Needs, Step};
pub use image::EffectImage;
pub use operation::{
    apply_effects, decode_recipe_v2, process, EffectsRequest, ProcessRequestV2, RecipeV2,
};
pub use recipe::{decode_effects, BuiltinEffect, EffectStep};
