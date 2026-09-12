use ditherette_wasm::{
    image::{ImageDimensions, ImageView, Rgba8},
    prod::{
        color::{
            rgba8_to_color_space_f32, rgba8_to_color_space_f32_rows_into, ColorSpaceF32,
            ColorTilingPolicy,
        },
        tiling::RowBand,
    },
};

const CHANNELS: usize = 4;

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}

fn fixture_rgba8(width: u32, height: u32) -> Vec<u8> {
    let mut source = Vec::with_capacity(width as usize * height as usize * CHANNELS);
    for y in 0..height {
        for x in 0..width {
            source.extend_from_slice(&[
                (x * 37 + y * 11) as u8,
                (x * 13 + y * 29) as u8,
                (x * 7 + y * 19) as u8,
                (64 + x * 3 + y * 5) as u8,
            ]);
        }
    }
    source
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

#[test]
fn prod_color_tiling_policy_keeps_tiny_images_scalar() {
    let dimensions = ImageDimensions::new(199, 201).unwrap();

    assert_eq!(
        ColorTilingPolicy::for_request(dimensions, ColorSpaceF32::LinearSrgb, true),
        None
    );
}

#[test]
fn prod_color_tiling_policy_uses_small_bands_after_parallel_threshold() {
    let dimensions = ImageDimensions::new(200, 200).unwrap();
    let policy = ColorTilingPolicy::for_request(dimensions, ColorSpaceF32::LinearSrgb, true)
        .expect("40k pixels should use color tiling");

    assert_eq!(policy.row_band_height(), 32);
}

#[test]
fn prod_color_tiling_policy_uses_larger_bands_for_large_perceptual_spaces() {
    let dimensions = ImageDimensions::new(1800, 1500).unwrap();

    assert_eq!(
        ColorTilingPolicy::for_request(dimensions, ColorSpaceF32::Oklab, true)
            .expect("large perceptual color conversion should use color tiling")
            .row_band_height(),
        64
    );
    assert_eq!(
        ColorTilingPolicy::for_request(dimensions, ColorSpaceF32::LinearSrgb, true)
            .expect("large linear sRGB conversion should use color tiling")
            .row_band_height(),
        32
    );
}

#[test]
fn prod_color_row_bands_match_full_image_output() {
    let dimensions = ImageDimensions::new(5, 4).unwrap();
    let source = fixture_rgba8(dimensions.width(), dimensions.height());
    let expected = rgba8_to_color_space_f32(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ColorSpaceF32::Oklab,
        false,
    );
    let mut actual = vec![f32::NAN; expected.len()];

    for band in [
        RowBand::new(0, 1).unwrap(),
        RowBand::new(1, 3).unwrap(),
        RowBand::new(3, 4).unwrap(),
    ] {
        rgba8_to_color_space_f32_rows_into(
            ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
            ColorSpaceF32::Oklab,
            band,
            &mut actual,
        );
    }

    assert_eq!(actual, expected);
}

#[test]
fn prod_color_row_band_writes_only_assigned_rows() {
    let dimensions = ImageDimensions::new(3, 3).unwrap();
    let source = fixture_rgba8(dimensions.width(), dimensions.height());
    let mut output = vec![-1.0; dimensions.pixel_count().unwrap() * CHANNELS];
    let band = RowBand::new(1, 2).unwrap();

    rgba8_to_color_space_f32_rows_into(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ColorSpaceF32::Srgb,
        band,
        &mut output,
    );

    let row_len = dimensions.width_usize() * CHANNELS;
    assert!(output[..row_len].iter().all(|value| *value == -1.0));
    assert!(output[row_len..row_len * 2]
        .iter()
        .all(|value| *value != -1.0));
    assert!(output[row_len * 2..].iter().all(|value| *value == -1.0));
}
