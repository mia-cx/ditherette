use ditherette_wasm::{image::ImageDimensions, resize::resize_rgba_nearest};

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}

#[test]
fn identity_resize_returns_same_bytes() {
    let source_rgba = vec![
        255, 0, 0, 255, // red
        0, 255, 0, 255, // green
        0, 0, 255, 255, // blue
        255, 255, 255, 255, // white
    ];

    let output_rgba =
        resize_rgba_nearest(&source_rgba, dimensions(2, 2), dimensions(2, 2)).unwrap();

    assert_eq!(output_rgba, source_rgba);
}

#[test]
fn one_pixel_source_fills_larger_output() {
    let source_rgba = vec![7, 11, 13, 255];

    let output_rgba =
        resize_rgba_nearest(&source_rgba, dimensions(1, 1), dimensions(3, 2)).unwrap();

    assert_eq!(output_rgba, source_rgba.repeat(6));
}

#[test]
fn two_by_two_upscale_repeats_each_pixel_into_blocks() {
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    let blue = [0, 0, 255, 255];
    let white = [255, 255, 255, 255];
    let source_rgba = [red, green, blue, white].concat();

    let output_rgba =
        resize_rgba_nearest(&source_rgba, dimensions(2, 2), dimensions(4, 4)).unwrap();

    let expected_rgba = [
        red, red, green, green, // row 0
        red, red, green, green, // row 1
        blue, blue, white, white, // row 2
        blue, blue, white, white, // row 3
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
}

#[test]
fn four_by_four_downscale_samples_top_left_proportional_pixels() {
    let source_rgba: Vec<u8> = (0..16)
        .flat_map(|value| [value, value, value, 255])
        .collect();

    let output_rgba =
        resize_rgba_nearest(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap();

    let expected_rgba = [
        [0, 0, 0, 255],
        [2, 2, 2, 255],
        [8, 8, 8, 255],
        [10, 10, 10, 255],
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
}

#[test]
fn non_square_resize_uses_independent_x_and_y_mapping() {
    let source_rgba: Vec<u8> = (0..6)
        .flat_map(|value| [value, value, value, 255])
        .collect();

    let output_rgba =
        resize_rgba_nearest(&source_rgba, dimensions(3, 2), dimensions(2, 3)).unwrap();

    let expected_rgba = [
        [0, 0, 0, 255],
        [1, 1, 1, 255],
        [0, 0, 0, 255],
        [1, 1, 1, 255],
        [3, 3, 3, 255],
        [4, 4, 4, 255],
    ]
    .concat();

    assert_eq!(output_rgba, expected_rgba);
}
