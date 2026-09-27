//! Coverage-aware resampling for recipe version 2.
//!
//! See `spec.md` for why colour is weighted by alpha and how it composes with v1.

pub mod resize;

pub use resize::resize;
