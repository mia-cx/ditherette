//! Literal f32 inverse fragments from the frozen linear reference.

use crate::image::{ImageFormat, ImageView, ImageViewMut, LinearRgb32, Rgba8};

use super::common::linear_to_srgb_unit;

/// Encode finite linear-light coordinates, then clip and round to RGB bytes.
/// Values outside the display gamut saturate; byte ties round upward.
pub fn linear_rgb_to_rgb8(linear: [f32; 3]) -> [u8; 3] {
    linear.map(|channel| (linear_to_srgb_unit(channel).clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Reconstruct RGBA8 using unchanged alpha bytes from the corresponding image.
/// All views must have equal dimensions; row padding is left untouched.
pub fn linear_rgb32_to_rgba8_into(
    source: ImageView<'_, LinearRgb32>,
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
            let source_start = x * LinearRgb32::CHANNEL_COUNT;
            let output_start = x * Rgba8::CHANNEL_COUNT;
            let rgb = linear_rgb_to_rgb8([
                source_row[source_start + LinearRgb32::R],
                source_row[source_start + LinearRgb32::G],
                source_row[source_start + LinearRgb32::B],
            ]);
            output_row[output_start..output_start + 3].copy_from_slice(&rgb);
            output_row[output_start + Rgba8::A] = alpha_row[output_start + Rgba8::A];
        }
    }
}
