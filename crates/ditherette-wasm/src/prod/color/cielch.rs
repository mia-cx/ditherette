//! Literal f32 inverse fragments from the frozen cielch reference.

use crate::image::{Cielch32, ImageFormat, ImageView, ImageViewMut, Rgba8};

use super::cielab::cielab_to_rgb8;

/// Reconstructs RGB from finite D65 CIELCH: negative chroma becomes neutral, hue wraps in radians.
pub fn cielch_to_rgb8([lightness, chroma, hue]: [f32; 3]) -> [u8; 3] {
    if chroma <= 0.0 {
        return cielab_to_rgb8([lightness, 0.0, 0.0]);
    }
    let hue = hue.rem_euclid(std::f32::consts::TAU);
    let hue = if hue == std::f32::consts::TAU {
        0.0
    } else {
        hue
    };
    cielab_to_rgb8([lightness, chroma * hue.cos(), chroma * hue.sin()])
}

/// Reconstructs RGB from packed D65 CIELCH and copies corresponding RGBA8 byte alpha unchanged.
pub fn cielch32_to_rgba8_into(
    source: ImageView<'_, Cielch32>,
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
            let color_start = x * Cielch32::CHANNEL_COUNT;
            let rgba_start = x * Rgba8::CHANNEL_COUNT;
            let rgb = cielch_to_rgb8([
                source_row[color_start],
                source_row[color_start + 1],
                source_row[color_start + 2],
            ]);
            output_row[rgba_start..rgba_start + 3].copy_from_slice(&rgb);
            output_row[rgba_start + Rgba8::A] = alpha_row[rgba_start + Rgba8::A];
        }
    }
}
