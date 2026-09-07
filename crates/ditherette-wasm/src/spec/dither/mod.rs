//! Executable specs for dithering in a selected working color space.
//!
//! These specs accept precomputed f32 working-space coordinates and palette
//! coordinates in the same working space, then emit `PaletteIndex8`. The current
//! oracle uses Euclidean nearest-palette matching in that dither working space;
//! the pipeline can still quantize separately with a different metric before or
//! after this layer as the product model evolves.

mod common;

pub mod blue_noise;
pub mod error_diffusion;
pub mod ordered;
pub mod placement;
pub mod random_noise;
pub mod yiluoma;
