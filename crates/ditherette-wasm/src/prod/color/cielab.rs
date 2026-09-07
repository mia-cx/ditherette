//! Literal f32 inverse fragments from the frozen cielab reference.

use crate::image::{Cielab32, ImageFormat, ImageView, ImageViewMut, Rgba8};

use super::common::{linear_to_srgb_unit, D65_XN, D65_YN, D65_ZN};

/// Reconstructs sRGB bytes from finite D65 CIELAB coordinates, clipping only final encoded RGB.
pub fn cielab_to_rgb8([lightness, a, b]: [f32; 3]) -> [u8; 3] {
    let fy = (lightness + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let x = D65_XN * inverse_lab_f(fx);
    let y = D65_YN * inverse_lab_f(fy);
    let z = D65_ZN * inverse_lab_f(fz);
    // Inverse of the inherited decimal sRGB-to-XYZ matrix, rounded to f32.
    let linear = [
        3.240_455 * x - 1.537_138_8 * y - 0.498_531_55 * z,
        -0.969_266_4 * x + 1.876_010_9 * y + 0.041_556_083 * z,
        0.055_643_42 * x - 0.204_025_85 * y + 1.057_225_1 * z,
    ];
    linear.map(|channel| (linear_to_srgb_unit(channel).clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn inverse_lab_f(value: f32) -> f32 {
    const DELTA: f32 = 6.0 / 29.0;
    const KAPPA: f32 = 24_389.0 / 27.0;
    if value > DELTA {
        value * value * value
    } else {
        (116.0 * value - 16.0) / KAPPA
    }
}

/// Reconstructs each RGB triplet and copies byte alpha from the corresponding RGBA8 source.
pub fn cielab32_to_rgba8_into(
    source: ImageView<'_, Cielab32>,
    alpha_source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, alpha_source.dimensions());
    assert_eq!(dimensions, output.dimensions());
    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let alpha_row = alpha_source.row(y).expect("alpha row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");
        for x in 0..dimensions.width_usize() {
            let color_start = x * Cielab32::CHANNEL_COUNT;
            let rgba_start = x * Rgba8::CHANNEL_COUNT;
            let rgb = cielab_to_rgb8([
                source_row[color_start],
                source_row[color_start + 1],
                source_row[color_start + 2],
            ]);
            output_row[rgba_start..rgba_start + 3].copy_from_slice(&rgb);
            output_row[rgba_start + Rgba8::A] = alpha_row[rgba_start + Rgba8::A];
        }
    }
}
