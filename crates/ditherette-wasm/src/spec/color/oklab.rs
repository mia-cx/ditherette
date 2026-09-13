//! Oklab conversion spec.

use crate::image::{ImageFormat, ImageView, ImageViewMut, Oklab32, Rgba8};

use super::common::{linear_to_srgb_unit, srgb8_to_oklab};

/// Converts encoded sRGB bytes to D65 Oklab with the inherited f32 matrix recipe.
pub fn rgb8_to_oklab([r, g, b]: [u8; 3]) -> [f32; 3] {
    srgb8_to_oklab(r, g, b)
}

/// Reconstructs sRGB bytes from finite Oklab coordinates, clipping only final encoded RGB.
pub fn oklab_to_rgb8([lightness, a, b]: [f32; 3]) -> [u8; 3] {
    // Ottosson's 2021-01-25 inverse, rounded to f32 like the forward matrix.
    let l_root = lightness + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_root = lightness - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_root = lightness - 0.089_484_18 * a - 1.291_485_5 * b;
    let l = l_root * l_root * l_root;
    let m = m_root * m_root * m_root;
    let s = s_root * s_root * s_root;
    let linear = [
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    ];
    linear.map(|channel| (linear_to_srgb_unit(channel).clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// Writes packed Oklab triples without interpreting or duplicating source alpha.
pub fn rgba8_to_oklab32_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Oklab32>) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * Rgba8::CHANNEL_COUNT;
            let output_start = x * Oklab32::CHANNEL_COUNT;
            let [l, a, b] = rgb8_to_oklab([
                source_row[source_start + Rgba8::R],
                source_row[source_start + Rgba8::G],
                source_row[source_start + Rgba8::B],
            ]);
            output_row[output_start + Oklab32::L] = l;
            output_row[output_start + Oklab32::A] = a;
            output_row[output_start + Oklab32::B] = b;
        }
    }
}

/// Reconstructs each RGB triplet and copies byte alpha from the corresponding RGBA8 source.
pub fn oklab32_to_rgba8_into(
    source: ImageView<'_, Oklab32>,
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
            let color_start = x * Oklab32::CHANNEL_COUNT;
            let rgba_start = x * Rgba8::CHANNEL_COUNT;
            let rgb = oklab_to_rgb8([
                source_row[color_start],
                source_row[color_start + 1],
                source_row[color_start + 2],
            ]);
            output_row[rgba_start..rgba_start + 3].copy_from_slice(&rgb);
            output_row[rgba_start + Rgba8::A] = alpha_row[rgba_start + Rgba8::A];
        }
    }
}
