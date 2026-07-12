//! Deterministic blue-noise threshold dithering spec.
//!
//! The MVP spec uses a bundled 8x8 high-frequency threshold tile. Production may
//! replace storage/layout, but exact blue-noise mode should preserve this tile or
//! explicitly version the threshold texture.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::common::{
    add_scaled_noise, assert_dither_inputs, nearest_euclidean, read_color, write_index, Palette3,
};

pub const BLUE_NOISE_8X8: [u8; 64] = [
    0, 48, 12, 60, 3, 51, 15, 63, 32, 16, 44, 28, 35, 19, 47, 31, 8, 56, 4, 52, 11, 59, 7, 55, 40,
    24, 36, 20, 43, 27, 39, 23, 2, 50, 14, 62, 1, 49, 13, 61, 34, 18, 46, 30, 33, 17, 45, 29, 10,
    58, 6, 54, 9, 57, 5, 53, 42, 26, 38, 22, 41, 25, 37, 21,
];

pub fn dither_blue_noise_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    strength: f32,
) {
    dither_blue_noise_by_nearest_into(source, palette, output, strength, nearest_euclidean);
}

pub fn dither_blue_noise_by_nearest_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    strength: f32,
    nearest: impl Fn([f32; 3], Palette3<'_>) -> (u8, [f32; 3]) + Copy,
) {
    assert_dither_inputs(&source, palette, &output);
    let dimensions = source.dimensions();

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let tile_index = (y as usize % 8) * 8 + (x % 8);
            let noise = ((BLUE_NOISE_8X8[tile_index] as f32 + 0.5) / 64.0) - 0.5;
            let color = add_scaled_noise(read_color::<F>(source_row, x), noise, strength);
            let (index, _) = nearest(color, palette);
            write_index(output_row, x, index);
        }
    }
}
