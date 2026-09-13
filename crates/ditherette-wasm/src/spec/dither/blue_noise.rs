//! Deterministic blue-noise threshold dithering spec.
//!
//! The periodic rank tile comes from the naive offline void-and-cluster reference.
//! The generator, construction gates, and provenance are frozen alongside the asset.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::common::{
    add_scaled_noise, assert_dither_inputs, nearest_euclidean, read_color, write_index, Palette3,
};

mod tile;
pub use tile::BLUE_NOISE_32X32;

pub const BLUE_NOISE_SIDE: u32 = 32;

/// Centered midpoint threshold in (-0.5, 0.5), indexed by complete-image coordinates.
/// One scalar sample is shared by the pixel's working-color channels. No palette is read.
pub fn blue_noise_at(x: u32, y: u32) -> f32 {
    let index = ((y % BLUE_NOISE_SIDE) * BLUE_NOISE_SIDE + x % BLUE_NOISE_SIDE) as usize;
    ((f32::from(BLUE_NOISE_32X32[index]) + 0.5) / BLUE_NOISE_32X32.len() as f32) - 0.5
}

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
            let noise = blue_noise_at(x as u32, y);
            let color = add_scaled_noise(read_color::<F>(source_row, x), noise, strength);
            let (index, _) = nearest(color, palette);
            write_index(output_row, x, index);
        }
    }
}
