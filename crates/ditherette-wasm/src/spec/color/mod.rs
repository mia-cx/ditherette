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
pub mod reconstruct;
pub mod spaces;
pub mod srgb;
pub mod ycbcr;

pub use common::{linear_to_srgb_unit, srgb8_to_linear, srgb8_to_unit, srgb_unit_to_linear};

use super::contract::request::WorkingSpace;

/// Converts encoded RGB bytes into the selected working coordinates, without alpha.
pub fn rgb8_to_coordinates(rgb: [u8; 3], space: WorkingSpace) -> [f32; 3] {
    match space {
        WorkingSpace::Srgb => srgb::rgb8_to_srgb(rgb),
        WorkingSpace::LinearRgb => linear::rgb8_to_linear_rgb(rgb),
        WorkingSpace::Oklab => oklab::rgb8_to_oklab(rgb),
        WorkingSpace::Oklch => oklch::rgb8_to_oklch(rgb),
        WorkingSpace::Cielab => cielab::rgb8_to_cielab(rgb),
        WorkingSpace::Cielch => cielch::rgb8_to_cielch(rgb),
        WorkingSpace::Ycbcr => ycbcr::rgb8_to_ycbcr(rgb),
    }
}

/// Reconstructs encoded RGB bytes using the selected space's clipping and rounding.
/// RGBA callers carry the original alpha byte independently.
pub fn coordinates_to_rgb8(coordinates: [f32; 3], space: WorkingSpace) -> [u8; 3] {
    match space {
        WorkingSpace::Srgb => srgb::srgb_to_rgb8(coordinates),
        WorkingSpace::LinearRgb => linear::linear_rgb_to_rgb8(coordinates),
        WorkingSpace::Oklab => oklab::oklab_to_rgb8(coordinates),
        WorkingSpace::Oklch => oklch::oklch_to_rgb8(coordinates),
        WorkingSpace::Cielab => cielab::cielab_to_rgb8(coordinates),
        WorkingSpace::Cielch => cielch::cielch_to_rgb8(coordinates),
        WorkingSpace::Ycbcr => ycbcr::ycbcr_to_rgb8(coordinates),
    }
}
