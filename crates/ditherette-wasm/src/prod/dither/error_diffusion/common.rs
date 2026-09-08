//! Shared helpers for spec dithering algorithms.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use crate::prod::quantize::metric::euclidean3_squared as squared_distance;

pub type Palette3<'a> = &'a [[f32; 3]];

pub fn assert_dither_inputs<F: ImageFormat<Storage = f32>>(
    source: &ImageView<'_, F>,
    palette: Palette3<'_>,
    output: &ImageViewMut<'_, PaletteIndex8>,
) {
    assert!(
        F::CHANNEL_COUNT >= 3,
        "dither specs require at least 3 channels"
    );
    assert!(
        !palette.is_empty(),
        "palette must contain at least one color"
    );
    assert!(palette.len() <= 256, "palette indices are stored as u8");
    assert_eq!(source.dimensions(), output.dimensions());
}

pub fn read_color<F: ImageFormat<Storage = f32>>(row: &[f32], x: usize) -> [f32; 3] {
    let start = x * F::CHANNEL_COUNT;
    [row[start], row[start + 1], row[start + 2]]
}

pub fn add_error(color: [f32; 3], error: [f32; 3]) -> [f32; 3] {
    [
        color[0] + error[0],
        color[1] + error[1],
        color[2] + error[2],
    ]
}

pub fn sub_color(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn nearest_euclidean(color: [f32; 3], palette: Palette3<'_>) -> (u8, [f32; 3]) {
    let mut best_index = 0;
    let mut best_distance = squared_distance(color, palette[0]);

    for (index, &candidate) in palette.iter().enumerate().skip(1) {
        let distance = squared_distance(color, candidate);
        if distance < best_distance {
            best_index = index;
            best_distance = distance;
        }
    }

    (best_index as u8, palette[best_index])
}

pub fn write_index(row: &mut [u8], x: usize, index: u8) {
    row[x * PaletteIndex8::CHANNEL_COUNT] = index;
}
