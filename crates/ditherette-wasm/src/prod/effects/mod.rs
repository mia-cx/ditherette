//! Production ordered effects, copied from the frozen `spec::effects` reference.
//!
//! Exact modes must match the reference byte-for-byte. See `README.md` for
//! optimizations and their evidence.

pub mod analysis_cache;
pub mod brightness_contrast;
pub mod chain;
pub mod channel;
pub mod curves;
pub mod exposure;
pub mod hue_saturation;
pub mod image;
pub mod levels;
pub mod memo;
pub mod operation;
pub mod recipe;
pub mod recolour;
pub mod recolour_analysis;
pub mod space;
pub mod table;
pub mod white_balance;

pub use chain::{apply_chain, Effect, EffectContext, Needs, Step};
pub use image::EffectImage;
pub use operation::{
    analyze_recolour, apply_effects, apply_in_place, carrier_after, carrier_bytes, AnalyzeRequest,
    EffectsRequest,
};
pub use recipe::{decode_effects, BuiltinEffect, EffectStep};
