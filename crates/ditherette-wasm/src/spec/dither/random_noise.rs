//! Seeded random-noise dithering spec.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::common::{
    add_scaled_noise, assert_dither_inputs, nearest_euclidean, read_color, write_index, Palette3,
};

const MULBERRY32_STEP: u32 = 0x6d2b_79f5;

/// The single Mulberry32 draw assigned to a global zero-based row-major pixel index.
/// Index arithmetic wraps modulo 2^32, independently of row bands and alpha.
pub fn random_u32_at(seed: u32, global_pixel_index: u64) -> u32 {
    let draw = (global_pixel_index as u32).wrapping_add(1);
    let state = seed.wrapping_add(draw.wrapping_mul(MULBERRY32_STEP));
    mulberry32_output(state)
}

/// Centered random threshold: divide the exact draw by 2^32, subtract 0.5, then round to f32.
pub fn random_noise_at(seed: u32, global_pixel_index: u64) -> f32 {
    (f64::from(random_u32_at(seed, global_pixel_index)) / 4_294_967_296.0 - 0.5) as f32
}

fn mulberry32_output(mut value: u32) -> u32 {
    value = (value ^ (value >> 15)).wrapping_mul(value | 1);
    value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 61));
    value ^ (value >> 14)
}

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
        self.state = self.state.wrapping_add(MULBERRY32_STEP);
        mulberry32_output(self.state)
    }

    pub fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / 4_294_967_296.0
    }
}
