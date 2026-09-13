//! sRGB conversion spec.
//!
//! This is the direct browser color space: RGBA8 input channels become normalized
//! gamma-encoded sRGB scalars in `0..=1` without linearization.

use crate::image::{ImageFormat, ImageView, ImageViewMut, Rgba8, Srgb32};

use super::srgb8_to_unit;

/// Normalize gamma-encoded RGB bytes to an sRGB triplet in `0..=1`.
pub fn rgb8_to_srgb(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(srgb8_to_unit)
}

/// Reconstruct RGB bytes from finite sRGB coordinates, clipping each channel.
/// Byte rounding uses nearest integer, with half-way values rounded upward.
pub fn srgb_to_rgb8(srgb: [f32; 3]) -> [u8; 3] {
    srgb.map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Convert RGB coordinates into packed triplets; source alpha stays in RGBA8.
pub fn rgba8_to_srgb32_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Srgb32>) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * Rgba8::CHANNEL_COUNT;
            let output_start = x * Srgb32::CHANNEL_COUNT;
            output_row[output_start + Srgb32::R] =
                srgb8_to_unit(source_row[source_start + Rgba8::R]);
            output_row[output_start + Srgb32::G] =
                srgb8_to_unit(source_row[source_start + Rgba8::G]);
            output_row[output_start + Srgb32::B] =
                srgb8_to_unit(source_row[source_start + Rgba8::B]);
        }
    }
}

/// Reconstruct RGBA8 using unchanged alpha bytes from the corresponding image.
/// All views must have equal dimensions; row padding is left untouched.
pub fn srgb32_to_rgba8_into(
    source: ImageView<'_, Srgb32>,
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
            let source_start = x * Srgb32::CHANNEL_COUNT;
            let output_start = x * Rgba8::CHANNEL_COUNT;
            let rgb = srgb_to_rgb8([
                source_row[source_start + Srgb32::R],
                source_row[source_start + Srgb32::G],
                source_row[source_start + Srgb32::B],
            ]);
            output_row[output_start..output_start + 3].copy_from_slice(&rgb);
            output_row[output_start + Rgba8::A] = alpha_row[output_start + Rgba8::A];
        }
    }
}
