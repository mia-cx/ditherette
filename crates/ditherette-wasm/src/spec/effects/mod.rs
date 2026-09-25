//! Ordered colour effects that run before palette output.
//!
//! See `spec.md` for the carrier, ordering, placement, and plugin boundary.

pub mod brightness_contrast;
pub mod chain;
pub mod channel;
pub mod curves;
pub mod exposure;
pub mod hue_saturation;
pub mod image;
pub mod levels;
pub mod operation;
pub mod recipe;
pub mod recolour;
pub mod recolour_analysis;
pub mod space;
pub mod white_balance;

pub use chain::{apply_chain, Effect, EffectContext, Needs, Step};
pub use image::EffectImage;
pub use operation::{
    analyze_recolour, apply_effects, decode_recipe_v2, process, AnalyzeRequest, EffectsRequest,
    ProcessRequestV2, RecipeV2,
};
pub use recipe::{decode_effects, BuiltinEffect, EffectStep};
