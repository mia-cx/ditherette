//! Frozen inverse-only transfer function and D65 white.

pub const D65_XN: f32 = 0.95047;
pub const D65_YN: f32 = 1.0;
pub const D65_ZN: f32 = 1.08883;

pub fn linear_to_srgb_unit(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        channel * 12.92
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}
