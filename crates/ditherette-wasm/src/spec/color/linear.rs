//! Linear sRGB conversion spec.

use crate::image::{ImageFormat, ImageView, ImageViewMut, LinearRgb32, Rgba8};

use super::srgb8_to_linear;

pub fn rgba8_to_linear_rgb32_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, LinearRgb32>,
) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * Rgba8::CHANNEL_COUNT;
            let output_start = x * LinearRgb32::CHANNEL_COUNT;
            output_row[output_start + LinearRgb32::R] =
                srgb8_to_linear(source_row[source_start + Rgba8::R]);
            output_row[output_start + LinearRgb32::G] =
                srgb8_to_linear(source_row[source_start + Rgba8::G]);
            output_row[output_start + LinearRgb32::B] =
                srgb8_to_linear(source_row[source_start + Rgba8::B]);
        }
    }
}
