use ditherette_wasm::{
    image::{
        ImageDimensions, ImageFormat, ImageView, ImageViewMut, LinearRgb32, Rgba8, RowStride,
        Srgb32, YCbCr32,
    },
    spec::color::{
        linear::{
            linear_rgb32_to_rgba8_into, linear_rgb_to_rgb8, rgb8_to_linear_rgb,
            rgba8_to_linear_rgb32_into,
        },
        linear_to_srgb_unit,
        srgb::{rgb8_to_srgb, rgba8_to_srgb32_into, srgb32_to_rgba8_into, srgb_to_rgb8},
        srgb_unit_to_linear,
        ycbcr::{rgb8_to_ycbcr, rgba8_to_ycbcr32_into, ycbcr32_to_rgba8_into, ycbcr_to_rgb8},
    },
};

fn assert_triplet_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 0.000_001,
            "{actual} differs from {expected}"
        );
    }
}

#[test]
fn srgb_keeps_encoded_values_and_rounds_byte_ties_upward() {
    assert_eq!(rgb8_to_srgb([0, 255, 128]), [0.0, 1.0, 128.0 / 255.0]);
    assert_eq!(srgb_to_rgb8([0.5, 0.5 / 255.0, 1.5 / 255.0]), [128, 1, 2]);
    assert_eq!(
        srgb_to_rgb8([0.5 - 0.000_001, 0.5, 0.5 + 0.000_001]),
        [127, 128, 128]
    );
    assert_eq!(srgb_to_rgb8([-0.25, 1.25, 0.0]), [0, 255, 0]);
}

#[test]
fn linear_uses_the_standard_piecewise_thresholds() {
    // Direct decimal evaluation of the IEC sRGB equations documented by W3C.
    for (encoded, linear) in [(0.040_45, 0.003_130_805), (0.040_451, 0.003_130_886)] {
        assert!((srgb_unit_to_linear(encoded) - linear).abs() < 0.000_000_003);
    }
    for (linear, encoded) in [(0.003_130_8, 0.040_449_936), (0.003_130_9, 0.040_451_18)] {
        assert!((linear_to_srgb_unit(linear) - encoded).abs() < 0.000_000_03);
    }
    assert_triplet_close(
        rgb8_to_linear_rgb([10, 11, 128]),
        [0.003_035_269_8, 0.003_346_535_8, 0.215_860_5],
    );
}

#[test]
fn linear_preserves_primaries_and_encodes_known_neutrals() {
    for rgb in [
        [0, 0, 0],
        [255, 255, 255],
        [255, 0, 0],
        [0, 255, 0],
        [0, 0, 255],
    ] {
        assert_triplet_close(
            rgb8_to_linear_rgb(rgb),
            rgb.map(|channel| channel as f32 / 255.0),
        );
    }
    // 18% and 50% linear light encode to 0.46135613 and 0.73535698 respectively.
    assert_eq!(linear_rgb_to_rgb8([0.18, 0.5, 1.0]), [118, 188, 255]);
    assert_eq!(linear_rgb_to_rgb8([-0.25, 2.0, 0.0]), [0, 255, 0]);
}

#[test]
fn ycbcr_matches_bt601_primary_vectors_with_full_range_chroma() {
    // BT.601 Table 1 luma and color differences, divided by 1.772/1.402 and shifted by 0.5.
    for (rgb, ycbcr) in [
        ([0, 0, 0], [0.0, 0.5, 0.5]),
        ([255, 255, 255], [1.0, 0.5, 0.5]),
        ([255, 0, 0], [0.299, 0.331_264_1, 1.0]),
        ([0, 255, 0], [0.587, 0.168_735_89, 0.081_312_41]),
        ([0, 0, 255], [0.114, 1.0, 0.418_687_58]),
    ] {
        assert_triplet_close(rgb8_to_ycbcr(rgb), ycbcr);
        assert_eq!(ycbcr_to_rgb8(ycbcr), rgb);
    }
}

#[test]
fn ycbcr_clips_only_reconstructed_rgb_and_preserves_neutral_ties() {
    assert_eq!(ycbcr_to_rgb8([0.5, 0.5, 0.5]), [128, 128, 128]);
    // At neutral luma, both minimum chroma coordinates reconstruct negative R/B and G > 1.
    assert_eq!(ycbcr_to_rgb8([0.5, 0.0, 0.0]), [0, 255, 0]);
    assert_eq!(ycbcr_to_rgb8([0.5, 1.0, 1.0]), [255, 0, 255]);
    assert_eq!(ycbcr_to_rgb8([0.0, 1.0, 0.5]), [0, 0, 226]);
    assert_eq!(ycbcr_to_rgb8([1.0, 0.0, 0.5]), [255, 255, 29]);
}

#[test]
fn every_byte_round_trips_in_neutral_and_single_channel_colors() {
    for byte in 0..=255 {
        for rgb in [[byte, byte, byte], [byte, 0, 0], [0, byte, 0], [0, 0, byte]] {
            assert_eq!(srgb_to_rgb8(rgb8_to_srgb(rgb)), rgb);
            assert_eq!(linear_rgb_to_rgb8(rgb8_to_linear_rgb(rgb)), rgb);
            assert_eq!(ycbcr_to_rgb8(rgb8_to_ycbcr(rgb)), rgb);
        }
    }
}

#[test]
fn mixed_colors_round_trip_across_transfer_boundaries() {
    let samples = [0, 1, 10, 11, 32, 64, 127, 128, 192, 254, 255];
    for r in samples {
        for g in samples {
            for b in samples {
                let rgb = [r, g, b];
                assert_eq!(srgb_to_rgb8(rgb8_to_srgb(rgb)), rgb);
                assert_eq!(linear_rgb_to_rgb8(rgb8_to_linear_rgb(rgb)), rgb);
                assert_eq!(ycbcr_to_rgb8(rgb8_to_ycbcr(rgb)), rgb);
            }
        }
    }
}

fn assert_strided_round_trip<F: ImageFormat<Storage = f32>>(
    forward: fn(ImageView<'_, Rgba8>, ImageViewMut<'_, F>),
    inverse: fn(ImageView<'_, F>, ImageView<'_, Rgba8>, ImageViewMut<'_, Rgba8>),
) {
    let dimensions = ImageDimensions::new(2, 2).unwrap();
    let source = [
        12, 34, 56, 201, 255, 0, 0, 202, 91, 92, 93, 0, 255, 0, 203, 0, 0, 255, 204,
    ];
    let alpha_source = [
        99, 98, 97, 0, 99, 98, 97, 7, 81, 82, 99, 98, 97, 128, 99, 98, 97, 255,
    ];
    let original_source = source;
    let original_alpha = alpha_source;
    let mut colors = [-123.0; 14];
    forward(
        ImageView::new(&source, dimensions, RowStride::new(11).unwrap()).unwrap(),
        ImageViewMut::new(&mut colors, dimensions, RowStride::new(8).unwrap()).unwrap(),
    );
    assert_eq!(&colors[6..8], &[-123.0; 2]);
    let original_colors = colors;
    let mut output = [77; 20];
    inverse(
        ImageView::new(&colors, dimensions, RowStride::new(8).unwrap()).unwrap(),
        ImageView::new(&alpha_source, dimensions, RowStride::new(10).unwrap()).unwrap(),
        ImageViewMut::new(&mut output, dimensions, RowStride::new(12).unwrap()).unwrap(),
    );
    assert_eq!(
        output,
        [12, 34, 56, 0, 255, 0, 0, 7, 77, 77, 77, 77, 0, 255, 0, 128, 0, 0, 255, 255]
    );
    assert_eq!(source, original_source);
    assert_eq!(alpha_source, original_alpha);
    assert_eq!(colors, original_colors);
}

#[test]
fn srgb_image_reconstruction_preserves_byte_alpha_and_padding() {
    assert_strided_round_trip::<Srgb32>(rgba8_to_srgb32_into, srgb32_to_rgba8_into);
}

#[test]
fn linear_image_reconstruction_preserves_byte_alpha_and_padding() {
    assert_strided_round_trip::<LinearRgb32>(
        rgba8_to_linear_rgb32_into,
        linear_rgb32_to_rgba8_into,
    );
}

#[test]
fn ycbcr_image_reconstruction_preserves_byte_alpha_and_padding() {
    assert_strided_round_trip::<YCbCr32>(rgba8_to_ycbcr32_into, ycbcr32_to_rgba8_into);
}
