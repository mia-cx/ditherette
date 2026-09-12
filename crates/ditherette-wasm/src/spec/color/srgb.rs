//! sRGB conversion spec.
//!
//! This is the direct browser color space: RGBA8 input channels become normalized
//! gamma-encoded sRGB scalars in `0..=1` without linearization.

use crate::image::{ImageFormat, ImageView, ImageViewMut, Rgba8, Srgb32};

use super::srgb8_to_unit;

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
