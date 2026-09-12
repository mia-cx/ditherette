use ditherette_wasm_old::{image::ImageDimensions, resize::resize_rgba_bilinear};

#[cfg(feature = "tiling")]
use ditherette_wasm_old::resize::{
    bilinear::{resize_rgba_bilinear_scalar_into, resize_rgba_bilinear_with_row_band_tiling_into},
    cpu_tiling::RowBandTiling,
};

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}

#[test]
fn one_pixel_source_fills_larger_output() {
    let source_rgba = vec![7, 11, 13, 255];

    let output_rgba =
        resize_rgba_bilinear(&source_rgba, dimensions(1, 1), dimensions(3, 2)).unwrap();

    assert_eq!(output_rgba, source_rgba.repeat(6));
}

#[test]
fn two_by_two_to_three_by_three_blends_center_aligned_pixels() {
    let source_rgba = [
        [0, 0, 0, 255],
        [100, 100, 100, 255],
        [200, 200, 200, 255],
        [255, 255, 255, 255],
    ]
    .concat();

    let output_rgba =
        resize_rgba_bilinear(&source_rgba, dimensions(2, 2), dimensions(3, 3)).unwrap();

    let expected_rgba = [
        [0, 0, 0, 255],
        [50, 50, 50, 255],
        [100, 100, 100, 255],
        [100, 100, 100, 255],
        [139, 139, 139, 255],
        [178, 178, 178, 255],
        [200, 200, 200, 255],
        [228, 228, 228, 255],
        [255, 255, 255, 255],
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
}

#[test]
fn four_by_four_to_two_by_two_matches_triangle_filter() {
    let source_rgba: Vec<u8> = (0..16)
        .flat_map(|value| [value, value, value, 255])
        .collect();

    let output_rgba =
        resize_rgba_bilinear(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap();

    let expected_rgba = [
        [4, 4, 4, 255],
        [5, 5, 5, 255],
        [10, 10, 10, 255],
        [11, 11, 11, 255],
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
}

#[test]
fn non_square_resize_uses_independent_x_and_y_weights() {
    let source_rgba = [[0, 0, 0, 255], [90, 90, 90, 255], [180, 180, 180, 255]].concat();

    let output_rgba =
        resize_rgba_bilinear(&source_rgba, dimensions(3, 1), dimensions(2, 2)).unwrap();

    let expected_rgba = [
        [34, 34, 34, 255],
        [146, 146, 146, 255],
        [34, 34, 34, 255],
        [146, 146, 146, 255],
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
}

#[cfg(feature = "tiling")]
#[test]
fn row_band_tiling_matches_scalar() {
    let source_dimensions = dimensions(17, 13);
    let output_dimensions = dimensions(11, 19);
    let source_rgba: Vec<u8> = (0..17 * 13 * 4)
        .map(|value| (value * 37 % 251) as u8)
        .collect();
    let mut scalar_rgba = vec![0; 11 * 19 * 4];
    let mut tiled_rgba = vec![0; 11 * 19 * 4];

    resize_rgba_bilinear_scalar_into(
        &source_rgba,
        source_dimensions,
        output_dimensions,
        &mut scalar_rgba,
    )
    .unwrap();
    resize_rgba_bilinear_with_row_band_tiling_into(
        &source_rgba,
        source_dimensions,
        output_dimensions,
        &mut tiled_rgba,
        RowBandTiling::new(0, 1, 1, 4),
    )
    .unwrap();

    assert_eq!(tiled_rgba, scalar_rgba);
}
