//! CIELAB conversion spec using D65 white and the sRGB transfer curve.

use crate::image::{Cielab32, ImageFormat, ImageView, ImageViewMut, Rgba8};

use super::common::srgb8_to_cielab;

pub fn rgba8_to_cielab32_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Cielab32>,
) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * Rgba8::CHANNEL_COUNT;
            let output_start = x * Cielab32::CHANNEL_COUNT;
            let [l, a, b] = srgb8_to_cielab(
                source_row[source_start + Rgba8::R],
                source_row[source_start + Rgba8::G],
                source_row[source_start + Rgba8::B],
            );
            output_row[output_start + Cielab32::L] = l;
            output_row[output_start + Cielab32::A] = a;
            output_row[output_start + Cielab32::B] = b;
        }
    }
}
