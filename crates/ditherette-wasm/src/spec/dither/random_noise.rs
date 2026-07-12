//! Seeded random-noise dithering spec.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::common::{
    add_scaled_noise, assert_dither_inputs, nearest_euclidean, read_color, write_index, Palette3,
};

pub fn dither_random_noise_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    seed: u32,
    strength: f32,
) {
    dither_random_noise_by_nearest_into(source, palette, output, seed, strength, nearest_euclidean);
}

pub fn dither_random_noise_by_nearest_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    seed: u32,
    strength: f32,
    nearest: impl Fn([f32; 3], Palette3<'_>) -> (u8, [f32; 3]) + Copy,
) {
    assert_dither_inputs(&source, palette, &output);
    let dimensions = source.dimensions();
    let mut rng = Mulberry32::new(seed);

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let noise = rng.next_f32() - 0.5;
            let color = add_scaled_noise(read_color::<F>(source_row, x), noise, strength);
            let (index, _) = nearest(color, palette);
            write_index(output_row, x, index);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut z = self.state;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add((z ^ (z >> 7)).wrapping_mul(z | 61));
        z ^ (z >> 14)
    }

    pub fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / 4_294_967_296.0
    }
}
