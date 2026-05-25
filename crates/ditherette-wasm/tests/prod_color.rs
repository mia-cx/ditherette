use ditherette_wasm::{
    image::{ImageDimensions, ImageView, Rgba8},
    prod::color::{rgba8_to_color_space_f32, ColorSpaceF32},
};

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}

#[test]
fn prod_color_materializes_four_channel_srgb_with_alpha() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [255, 128, 0, 64];

    let output = rgba8_to_color_space_f32(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ColorSpaceF32::Srgb,
        false,
    );

    assert_eq!(output.len(), 4);
    assert_close(output[0], 1.0, f32::EPSILON);
    assert_close(output[1], 128.0 / 255.0, f32::EPSILON);
    assert_close(output[2], 0.0, f32::EPSILON);
    assert_close(output[3], 64.0 / 255.0, f32::EPSILON);
}

#[test]
fn prod_color_materializes_known_oklab_and_cielab_values() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let red = [255, 0, 0, 255];

    let oklab = rgba8_to_color_space_f32(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ColorSpaceF32::Oklab,
        true,
    );
    assert_close(oklab[0], 0.627_955, 0.000_001);
    assert_close(oklab[1], 0.224_863, 0.000_001);
    assert_close(oklab[2], 0.125_846, 0.000_001);
    assert_close(oklab[3], 1.0, f32::EPSILON);

    let cielab = rgba8_to_color_space_f32(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ColorSpaceF32::Cielab,
        true,
    );
    assert_close(cielab[0], 53.240_8, 0.001);
    assert_close(cielab[1], 80.092_5, 0.001);
    assert_close(cielab[2], 67.203_2, 0.001);
    assert_close(cielab[3], 1.0, f32::EPSILON);
}

#[test]
fn prod_color_materializes_cylindrical_and_ycbcr_spaces() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let red = [255, 0, 0, 128];

    let oklch = rgba8_to_color_space_f32(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ColorSpaceF32::Oklch,
        false,
    );
    assert_close(oklch[0], 0.627_955, 0.000_001);
    assert_close(oklch[1], 0.257_683, 0.000_001);
    assert_close(oklch[3], 128.0 / 255.0, f32::EPSILON);

    let ycbcr = rgba8_to_color_space_f32(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ColorSpaceF32::YCbCr,
        false,
    );
    assert_close(ycbcr[0], 0.299, 0.000_001);
    assert_close(ycbcr[1], 0.331_264, 0.000_001);
    assert_close(ycbcr[2], 1.0, 0.000_001);
    assert_close(ycbcr[3], 128.0 / 255.0, f32::EPSILON);
}
