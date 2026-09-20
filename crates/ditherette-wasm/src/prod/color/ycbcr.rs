//! Literal f32 inverse fragments from the frozen ycbcr reference.

use crate::image::{ImageFormat, ImageView, ImageViewMut, Rgba8, YCbCr32};

/// Reconstruct gamma-encoded RGB, then clip and round each channel to a byte.
/// Valid full-range triplets may be outside the sRGB gamut. Byte ties round up.
pub fn ycbcr_to_rgb8([y, cb, cr]: [f32; 3]) -> [u8; 3] {
    let r = y + 1.402 * (cr - 0.5);
    let b = y + 1.772 * (cb - 0.5);
    let g = y - (0.114 * 1.772 * (cb - 0.5) + 0.299 * 1.402 * (cr - 0.5)) / 0.587;
    [r, g, b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Reconstruct RGBA8 using unchanged alpha bytes from the corresponding image.
/// All views must have equal dimensions; row padding is left untouched.
pub fn ycbcr32_to_rgba8_into(
    source: ImageView<'_, YCbCr32>,
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
            let source_start = x * YCbCr32::CHANNEL_COUNT;
            let output_start = x * Rgba8::CHANNEL_COUNT;
            let rgb = ycbcr_to_rgb8([
                source_row[source_start + YCbCr32::Y],
                source_row[source_start + YCbCr32::CB],
                source_row[source_start + YCbCr32::CR],
            ]);
            output_row[output_start..output_start + 3].copy_from_slice(&rgb);
            output_row[output_start + Rgba8::A] = alpha_row[output_start + Rgba8::A];
        }
    }
}
