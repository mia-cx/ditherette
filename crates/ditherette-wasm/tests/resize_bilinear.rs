use ditherette_wasm::{image::ImageDimensions, resize::resize_rgba_bilinear};

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
