//! CIELCH conversion spec: cylindrical CIELAB as L, chroma, hue radians.

use crate::image::{Cielch32, ImageFormat, ImageView, ImageViewMut, Rgba8};

use super::common::{cartesian_to_cylindrical, srgb8_to_cielab};

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
            let [l, a, b] = srgb8_to_cielab(
                source_row[source_start + Rgba8::R],
                source_row[source_start + Rgba8::G],
                source_row[source_start + Rgba8::B],
            );
            let [lightness, chroma, hue] = cartesian_to_cylindrical(l, a, b);
            output_row[output_start + Cielch32::L] = lightness;
            output_row[output_start + Cielch32::C] = chroma;
            output_row[output_start + Cielch32::H] = hue;
        }
    }
}
