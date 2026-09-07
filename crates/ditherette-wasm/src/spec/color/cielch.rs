//! CIELCH conversion spec: cylindrical CIELAB as L, chroma, hue radians.

use crate::image::{Cielch32, ImageFormat, ImageView, ImageViewMut, Rgba8};

use super::{
    cielab::{cielab_to_rgb8, rgb8_to_cielab},
    common::cartesian_to_cylindrical,
};

/// Converts sRGB bytes to D65 CIELCH, with exact byte grays represented by zero chroma and hue.
pub fn rgb8_to_cielch(rgb: [u8; 3]) -> [f32; 3] {
    let [lightness, a, b] = rgb8_to_cielab(rgb);
    if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
        return [lightness, 0.0, 0.0];
    }
    let [lightness, chroma, hue] = cartesian_to_cylindrical(lightness, a, b);
    let hue = if hue == 0.0 || hue == std::f32::consts::TAU {
        0.0
    } else {
        hue
    };
    [lightness, chroma, hue]
}

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

/// Writes packed CIELCH triples with hue in `[0, TAU)` and alpha kept in the source image.
pub fn rgba8_to_cielch32_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Cielch32>,
) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * Rgba8::CHANNEL_COUNT;
            let output_start = x * Cielch32::CHANNEL_COUNT;
            let [lightness, chroma, hue] = rgb8_to_cielch([
                source_row[source_start + Rgba8::R],
                source_row[source_start + Rgba8::G],
                source_row[source_start + Rgba8::B],
            ]);
            output_row[output_start + Cielch32::L] = lightness;
            output_row[output_start + Cielch32::C] = chroma;
            output_row[output_start + Cielch32::H] = hue;
        }
    }
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
