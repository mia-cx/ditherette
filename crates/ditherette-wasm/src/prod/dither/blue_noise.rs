//! Literal palette-free blue-noise field fragments from the frozen reference.

mod tile;
pub use tile::BLUE_NOISE_32X32;

pub const BLUE_NOISE_SIDE: u32 = 32;

/// Centered midpoint threshold in (-0.5, 0.5), indexed by complete-image coordinates.
/// One scalar sample is shared by the pixel's working-color channels. No palette is read.
pub fn blue_noise_at(x: u32, y: u32) -> f32 {
    let index = ((y % BLUE_NOISE_SIDE) * BLUE_NOISE_SIDE + x % BLUE_NOISE_SIDE) as usize;
    ((f32::from(BLUE_NOISE_32X32[index]) + 0.5) / BLUE_NOISE_32X32.len() as f32) - 0.5
}
