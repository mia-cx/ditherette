//! Literal global-index random field fragments. Legacy stream rounding is not used here.

const MULBERRY32_STEP: u32 = 0x6d2b_79f5;

/// The single Mulberry32 draw assigned to a global zero-based row-major pixel index.
/// Index arithmetic wraps modulo 2^32, independently of row bands and alpha.
pub fn random_u32_at(seed: u32, global_pixel_index: u64) -> u32 {
    let draw = (global_pixel_index as u32).wrapping_add(1);
    let state = seed.wrapping_add(draw.wrapping_mul(MULBERRY32_STEP));
    mulberry32_output(state)
}

/// Centered random threshold: divide the exact draw by 2^32, subtract 0.5, then round to f32.
pub fn random_noise_at(seed: u32, global_pixel_index: u64) -> f32 {
    (f64::from(random_u32_at(seed, global_pixel_index)) / 4_294_967_296.0 - 0.5) as f32
}

fn mulberry32_output(mut value: u32) -> u32 {
    value = (value ^ (value >> 15)).wrapping_mul(value | 1);
    value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
    value ^ (value >> 14)
}
