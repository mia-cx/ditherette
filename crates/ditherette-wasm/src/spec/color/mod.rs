//! Executable specs for converting browser RGBA8 image data into color spaces.
//!
//! Input is treated as non-HDR, display-referred sRGB in browser `ImageData`
//! RGBA8 order. Alpha is copied only by explicit RGBA/alpha formats elsewhere;
//! these color-space views expose color coordinates for matching, grading, and
//! later quantization stages.

mod common;

pub mod cielab;
pub mod cielch;
pub mod lab_ciede2000;
pub mod linear;
pub mod oklab;
pub mod oklch;
pub mod spaces;
pub mod srgb;
pub mod ycbcr;

pub use common::{linear_to_srgb_unit, srgb8_to_linear, srgb8_to_unit, srgb_unit_to_linear};
