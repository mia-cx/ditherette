//! Nearest-palette quantization specs.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::metric::{
    ciede2000_distance, circular_hue3_squared, euclidean3_squared, weighted_rgb_squared,
    WeightedRgbMetric,
};

pub type Palette3<'a> = &'a [[f32; 3]];

/// Quantize a 3-channel f32 color-coordinate image with squared Euclidean distance.
pub fn quantize_euclidean3_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
) {
    quantize_by_squared_distance_into(source, palette, output, euclidean3_squared);
}

/// Quantize L/C/h color-coordinate images using circular hue distance.
pub fn quantize_circular_hue3_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
) {
    quantize_by_squared_distance_into(source, palette, output, circular_hue3_squared);
}

/// Quantize gamma-encoded sRGB coordinates using an RGB-specific weighted metric.
pub fn quantize_weighted_rgb_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    metric: WeightedRgbMetric,
) {
    quantize_by_squared_distance_into(source, palette, output, |source, palette| {
        weighted_rgb_squared(source, palette, metric)
    });
}

/// Quantize CIELAB coordinates using CIEDE2000.
pub fn quantize_ciede2000_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
) {
    quantize_by_distance_into(source, palette, output, ciede2000_distance);
}

pub fn nearest_euclidean3_index(color: [f32; 3], palette: Palette3<'_>) -> u8 {
    nearest_by_distance(color, palette, euclidean3_squared)
}

pub fn nearest_circular_hue3_index(color: [f32; 3], palette: Palette3<'_>) -> u8 {
    nearest_by_distance(color, palette, circular_hue3_squared)
}

pub fn nearest_weighted_rgb_index(
    color: [f32; 3],
    palette: Palette3<'_>,
    metric: WeightedRgbMetric,
) -> u8 {
    nearest_by_distance(color, palette, |source, palette| {
        weighted_rgb_squared(source, palette, metric)
    })
}

pub fn nearest_ciede2000_index(color: [f32; 3], palette: Palette3<'_>) -> u8 {
    nearest_by_distance(color, palette, ciede2000_distance)
}

fn quantize_by_squared_distance_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
) {
    quantize_by_distance_into(source, palette, output, distance);
}

fn quantize_by_distance_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
) {
    assert!(
        F::CHANNEL_COUNT >= 3,
        "quantize specs require at least 3 channels"
    );
    assert_valid_palette(palette);
    assert_eq!(source.dimensions(), output.dimensions());

    let dimensions = source.dimensions();
    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * F::CHANNEL_COUNT;
            let color = [
                source_row[source_start],
                source_row[source_start + 1],
                source_row[source_start + 2],
            ];
            output_row[x] = nearest_by_distance(color, palette, distance);
        }
    }
}

fn nearest_by_distance(
    color: [f32; 3],
    palette: Palette3<'_>,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32,
) -> u8 {
    assert_valid_palette(palette);

    let mut best_index = 0;
    let mut best_distance = distance(color, palette[0]);

    for (index, &candidate) in palette.iter().enumerate().skip(1) {
        let candidate_distance = distance(color, candidate);
        if candidate_distance < best_distance {
            best_index = index;
            best_distance = candidate_distance;
        }
    }

    best_index as u8
}

fn assert_valid_palette(palette: Palette3<'_>) {
    assert!(
        !palette.is_empty(),
        "palette must contain at least one color"
    );
    assert!(palette.len() <= 256, "palette indices are stored as u8");
}
