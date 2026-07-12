//! Ordered Bayer dithering specs.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::common::{
    add_scaled_noise, assert_dither_inputs, nearest_euclidean, read_color, write_index, Palette3,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BayerSize {
    Two,
    Four,
    Eight,
    Sixteen,
}

impl BayerSize {
    pub const fn width(self) -> usize {
        match self {
            Self::Two => 2,
            Self::Four => 4,
            Self::Eight => 8,
            Self::Sixteen => 16,
        }
    }
}

pub fn dither_bayer_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    size: BayerSize,
    strength: f32,
) {
    dither_bayer_by_nearest_into(source, palette, output, size, strength, nearest_euclidean);
}

pub fn dither_bayer_by_nearest_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    size: BayerSize,
    strength: f32,
    nearest: impl Fn([f32; 3], Palette3<'_>) -> (u8, [f32; 3]) + Copy,
) {
    assert_dither_inputs(&source, palette, &output);
    let dimensions = source.dimensions();
    let width = size.width();
    let denominator = (width * width) as f32;

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let threshold = bayer_value(x % width, y as usize % width, width) as f32;
            let noise = ((threshold + 0.5) / denominator) - 0.5;
            let color = add_scaled_noise(read_color::<F>(source_row, x), noise, strength);
            let (index, _) = nearest(color, palette);
            write_index(output_row, x, index);
        }
    }
}

pub fn bayer_value(x: usize, y: usize, width: usize) -> u16 {
    assert!(width.is_power_of_two());
    assert!((2..=16).contains(&width));
    let mut value = 0;
    let mut bit = 1;
    while bit < width {
        let rx = usize::from((x & bit) != 0);
        let ry = usize::from((y & bit) != 0);
        value = (value << 2) | ((rx ^ ry) << 1) | ry;
        bit <<= 1;
    }
    value as u16
}
