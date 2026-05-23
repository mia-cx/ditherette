use ditherette_wasm::{
    image::{
        Cielab32, Cielch32, ImageDimensions, ImageView, ImageViewMut, LinearRgb32, Oklab32,
        Oklch32, Rgba8, Srgb32, YCbCr32,
    },
    spec::color::{
        cielab::rgba8_to_cielab32_into, cielch::rgba8_to_cielch32_into, lab_ciede2000::ciede2000,
        linear::rgba8_to_linear_rgb32_into, oklab::rgba8_to_oklab32_into,
        oklch::rgba8_to_oklch32_into, srgb::rgba8_to_srgb32_into, ycbcr::rgba8_to_ycbcr32_into,
    },
};

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}

#[test]
fn srgb_conversion_normalizes_gamma_encoded_channels() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [255, 128, 0, 7];
    let mut output = [0.0; 3];

    rgba8_to_srgb32_into(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ImageViewMut::<Srgb32>::packed(&mut output, dimensions).unwrap(),
    );

    assert_close(output[0], 1.0, f32::EPSILON);
    assert_close(output[1], 128.0 / 255.0, f32::EPSILON);
    assert_close(output[2], 0.0, f32::EPSILON);
}

#[test]
fn linear_conversion_applies_srgb_transfer_curve() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [255, 128, 0, 255];
    let mut output = [0.0; 3];

    rgba8_to_linear_rgb32_into(
        ImageView::<Rgba8>::packed(&source, dimensions).unwrap(),
        ImageViewMut::<LinearRgb32>::packed(&mut output, dimensions).unwrap(),
    );

    assert_close(output[0], 1.0, f32::EPSILON);
    assert_close(output[1], 0.215_860_53, 0.000_001);
    assert_close(output[2], 0.0, f32::EPSILON);
}

#[test]
fn oklab_and_oklch_convert_from_linearized_srgb() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let red = [255, 0, 0, 255];
    let mut lab = [0.0; 3];
    let mut lch = [0.0; 3];

    rgba8_to_oklab32_into(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ImageViewMut::<Oklab32>::packed(&mut lab, dimensions).unwrap(),
    );
    rgba8_to_oklch32_into(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ImageViewMut::<Oklch32>::packed(&mut lch, dimensions).unwrap(),
    );

    assert_close(lab[0], 0.627_955, 0.000_001);
    assert_close(lab[1], 0.224_863, 0.000_001);
    assert_close(lab[2], 0.125_846, 0.000_001);
    assert_close(lch[0], lab[0], f32::EPSILON);
    assert_close(
        lch[1],
        (lab[1] * lab[1] + lab[2] * lab[2]).sqrt(),
        0.000_001,
    );
    assert_close(lch[2], lab[2].atan2(lab[1]), 0.000_001);
}

#[test]
fn cielab_and_cielch_use_d65_lab_coordinates() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let red = [255, 0, 0, 255];
    let mut lab = [0.0; 3];
    let mut lch = [0.0; 3];

    rgba8_to_cielab32_into(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ImageViewMut::<Cielab32>::packed(&mut lab, dimensions).unwrap(),
    );
    rgba8_to_cielch32_into(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ImageViewMut::<Cielch32>::packed(&mut lch, dimensions).unwrap(),
    );

    assert_close(lab[0], 53.240_8, 0.001);
    assert_close(lab[1], 80.092_5, 0.001);
    assert_close(lab[2], 67.203_2, 0.001);
    assert_close(lch[0], lab[0], f32::EPSILON);
    assert_close(lch[1], (lab[1] * lab[1] + lab[2] * lab[2]).sqrt(), 0.001);
}

#[test]
fn ciede2000_matches_published_reference_pair() {
    let delta = ciede2000([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485]);

    assert_close(delta, 2.0425, 0.0001);
}

#[test]
fn ycbcr_uses_full_range_bt601_over_srgb() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let red = [255, 0, 0, 255];
    let mut output = [0.0; 3];

    rgba8_to_ycbcr32_into(
        ImageView::<Rgba8>::packed(&red, dimensions).unwrap(),
        ImageViewMut::<YCbCr32>::packed(&mut output, dimensions).unwrap(),
    );

    assert_close(output[0], 0.299, 0.000_001);
    assert_close(output[1], 0.331_264, 0.000_001);
    assert_close(output[2], 1.0, 0.000_001);
}
