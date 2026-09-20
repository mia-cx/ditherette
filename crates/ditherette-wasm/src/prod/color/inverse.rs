//! Original f32 inverse dispatch, distinct from wide scalar field reconstruction.

use super::{cielab, cielch, linear, oklab, oklch, srgb, ycbcr};
use crate::prod::contract::request::WorkingSpace;

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
