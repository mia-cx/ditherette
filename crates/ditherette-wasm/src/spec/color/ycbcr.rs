//! YCbCr conversion spec.
//!
//! Uses full-range BT.601 coefficients over gamma-encoded sRGB components:
//! `Y = 0.299R + 0.587G + 0.114B`, with neutral chroma at `0.5`.

use crate::image::{ImageFormat, ImageView, ImageViewMut, Rgba8, YCbCr32};

use super::srgb8_to_unit;

pub fn rgba8_to_ycbcr32_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, YCbCr32>) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * Rgba8::CHANNEL_COUNT;
            let output_start = x * YCbCr32::CHANNEL_COUNT;
            let r = srgb8_to_unit(source_row[source_start + Rgba8::R]);
            let g = srgb8_to_unit(source_row[source_start + Rgba8::G]);
            let b = srgb8_to_unit(source_row[source_start + Rgba8::B]);
            let y = 0.299 * r + 0.587 * g + 0.114 * b;
            let cb = 0.5 + (b - y) / 1.772;
            let cr = 0.5 + (r - y) / 1.402;

            output_row[output_start + YCbCr32::Y] = y;
            output_row[output_start + YCbCr32::CB] = cb;
            output_row[output_start + YCbCr32::CR] = cr;
        }
    }
}
