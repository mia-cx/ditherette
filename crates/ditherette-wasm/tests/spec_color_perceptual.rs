use ditherette_wasm::{
    image::{
        Cielab32, Cielch32, ImageDimensions, ImageFormat, ImageView, ImageViewMut, Oklab32,
        Oklch32, Rgba8, RowStride,
    },
    spec::color::{
        cielab::{cielab32_to_rgba8_into, cielab_to_rgb8, rgb8_to_cielab, rgba8_to_cielab32_into},
        cielch::{cielch32_to_rgba8_into, cielch_to_rgb8, rgb8_to_cielch, rgba8_to_cielch32_into},
        oklab::{oklab32_to_rgba8_into, oklab_to_rgb8, rgb8_to_oklab, rgba8_to_oklab32_into},
        oklch::{oklch32_to_rgba8_into, oklch_to_rgb8, rgb8_to_oklch, rgba8_to_oklch32_into},
    },
};

fn assert_coordinates(actual: [f32; 3], expected: [f32; 3], tolerance: f32) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} differs from {expected}"
        );
    }
}

#[test]
fn oklab_matches_primary_vectors_from_ottossons_published_matrix() {
    // https://bottosson.github.io/posts/oklab/#converting-from-linear-srgb-to-oklab
    // Independently evaluated in f64; RGB primaries are already linear endpoints.
    let cases = [
        ([255, 0, 0], [0.627_955_4, 0.224_863_07, 0.125_846_3]),
        ([0, 255, 0], [0.866_439_64, -0.233_887_57, 0.179_498_48]),
        ([0, 0, 255], [0.452_013_73, -0.032_456_983, -0.311_528_15]),
    ];
    for (rgb, lab) in cases {
        assert_coordinates(rgb8_to_oklab(rgb), lab, 0.000_001);
        assert_eq!(oklab_to_rgb8(lab), rgb);
    }
}

#[test]
fn cielab_matches_independently_evaluated_d65_primary_vectors() {
    // W3C's piecewise Lab formula with this recipe's D65 white and inherited XYZ matrix.
    // https://www.w3.org/TR/css-color-4/#color-conversion-code
    let cases = [
        ([255, 0, 0], [53.240_795, 80.092_46, 67.203_19]),
        ([0, 255, 0], [87.734_726, -86.182_72, 83.179_32]),
        ([0, 0, 255], [32.297_012, 79.187_52, -107.860_16]),
    ];
    for (rgb, lab) in cases {
        assert_coordinates(rgb8_to_cielab(rgb), lab, 0.000_05);
        assert_eq!(cielab_to_rgb8(lab), rgb);
    }
}

#[test]
fn cartesian_round_trips_cover_every_gray_and_a_stratified_rgb_cube() {
    for gray in 0..=255 {
        let rgb = [gray; 3];
        assert_eq!(oklab_to_rgb8(rgb8_to_oklab(rgb)), rgb);
        assert_eq!(cielab_to_rgb8(rgb8_to_cielab(rgb)), rgb);
    }
    // Includes sRGB's encoded transfer breakpoint between bytes 10 and 11.
    let channels = [0, 1, 10, 11, 32, 64, 127, 128, 192, 254, 255];
    for r in channels {
        for g in channels {
            for b in channels {
                let rgb = [r, g, b];
                assert_eq!(oklab_to_rgb8(rgb8_to_oklab(rgb)), rgb, "Oklab {rgb:?}");
                assert_eq!(cielab_to_rgb8(rgb8_to_cielab(rgb)), rgb, "CIELAB {rgb:?}");
            }
        }
    }
}

#[test]
fn inverse_lightness_extremes_clip_at_the_rgb_boundary() {
    for lightness in [-1.0, -0.1] {
        assert_eq!(oklab_to_rgb8([lightness, 0.0, 0.0]), [0; 3]);
    }
    for lightness in [1.1, 2.0] {
        assert_eq!(oklab_to_rgb8([lightness, 0.0, 0.0]), [255; 3]);
    }
    for lightness in [-100.0, -1.0] {
        assert_eq!(cielab_to_rgb8([lightness, 0.0, 0.0]), [0; 3]);
    }
    for lightness in [110.0, 200.0] {
        assert_eq!(cielab_to_rgb8([lightness, 0.0, 0.0]), [255; 3]);
    }
}

#[test]
fn chromatic_out_of_gamut_values_clip_channels_independently() {
    // Independent f64 evaluation of Ottosson's inverse has encoded R>1 and G<0.
    assert_eq!(oklab_to_rgb8([0.7, 0.4, 0.0]), [255, 0, 148]);
    assert_eq!(cielab_to_rgb8([50.0, 150.0, 100.0]), [255, 0, 0]);
}

#[test]
fn cielab_inverse_crosses_the_linear_cubic_threshold_at_lightness_eight() {
    // The inverse neutral Y is L/kappa below L=8, and ((L+16)/116)^3 above it.
    // Independently applying sRGB encoding gives bytes 23, 24, and 25.
    for (lightness, byte) in [(7.5, 23), (8.0, 24), (9.0, 25)] {
        assert_eq!(cielab_to_rgb8([lightness, 0.0, 0.0]), [byte; 3]);
    }
}

#[test]
fn cylindrical_grays_have_canonical_zero_chroma_and_hue() {
    for gray in 0..=255 {
        let rgb = [gray; 3];
        for (lch, restored) in [
            (rgb8_to_oklch(rgb), oklch_to_rgb8(rgb8_to_oklch(rgb))),
            (rgb8_to_cielch(rgb), cielch_to_rgb8(rgb8_to_cielch(rgb))),
        ] {
            assert_eq!(lch[1].to_bits(), 0.0_f32.to_bits());
            assert_eq!(lch[2].to_bits(), 0.0_f32.to_bits());
            assert_eq!(restored, rgb);
        }
    }
}

#[test]
fn cylindrical_near_grays_retain_chroma_and_all_sampled_colors_round_trip() {
    for rgb in [[127, 128, 128], [255, 254, 255], [0, 0, 1]] {
        assert!(rgb8_to_oklch(rgb)[1] > 0.0);
        assert!(rgb8_to_cielch(rgb)[1] > 0.0);
    }
    for r in [0, 1, 10, 11, 127, 128, 254, 255] {
        for g in [0, 1, 10, 11, 127, 128, 254, 255] {
            for b in [0, 1, 10, 11, 127, 128, 254, 255] {
                let rgb = [r, g, b];
                let oklch = rgb8_to_oklch(rgb);
                let cielch = rgb8_to_cielch(rgb);
                assert!((0.0..std::f32::consts::TAU).contains(&oklch[2]));
                assert!((0.0..std::f32::consts::TAU).contains(&cielch[2]));
                assert_eq!(oklch_to_rgb8(oklch), rgb, "OKLCH {rgb:?}");
                assert_eq!(cielch_to_rgb8(cielch), rgb, "CIELCH {rgb:?}");
            }
        }
    }
}

#[test]
fn cylindrical_hue_wraps_across_the_seam_and_negative_chroma_is_neutral() {
    let tau = std::f32::consts::TAU;
    for hue in [0.0, -0.0, tau, -tau, 2.0 * tau, -2.0 * tau, -0.000_000_1] {
        assert_eq!(
            oklch_to_rgb8([0.6, 0.15, hue]),
            oklab_to_rgb8([0.6, 0.15, 0.0])
        );
        assert_eq!(
            cielch_to_rgb8([60.0, 30.0, hue]),
            cielab_to_rgb8([60.0, 30.0, 0.0])
        );
    }
    for chroma in [0.0, -0.0, -1.0] {
        for hue in [0.0, 1.0, -5.0] {
            assert_eq!(
                oklch_to_rgb8([0.6, chroma, hue]),
                oklab_to_rgb8([0.6, 0.0, 0.0])
            );
            assert_eq!(
                cielch_to_rgb8([60.0, chroma, hue]),
                cielab_to_rgb8([60.0, 0.0, 0.0])
            );
        }
    }
}

fn check_image_round_trip<F: ImageFormat<Storage = f32>>(
    forward: fn(ImageView<'_, Rgba8>, ImageViewMut<'_, F>),
    inverse: fn(ImageView<'_, F>, ImageView<'_, Rgba8>, ImageViewMut<'_, Rgba8>),
) {
    let dimensions = ImageDimensions::new(2, 2).unwrap();
    let rgba_stride = RowStride::new(12).unwrap();
    let color_stride = RowStride::new(8).unwrap();
    let output_stride = RowStride::new(10).unwrap();
    let source = [
        255, 0, 0, 7, 10, 10, 10, 3, 99, 99, 99, 99, 0, 255, 0, 8, 0, 0, 255, 4, 99, 99, 99, 99,
    ];
    let alpha_source = [
        1, 2, 3, 0, 1, 2, 3, 64, 99, 99, 99, 99, 1, 2, 3, 128, 1, 2, 3, 255, 99, 99, 99, 99,
    ];
    let mut colors = [-999.0; 16];
    let mut output = [238; 20];
    forward(
        ImageView::new(&source, dimensions, rgba_stride).unwrap(),
        ImageViewMut::new(&mut colors, dimensions, color_stride).unwrap(),
    );
    inverse(
        ImageView::new(&colors, dimensions, color_stride).unwrap(),
        ImageView::new(&alpha_source, dimensions, rgba_stride).unwrap(),
        ImageViewMut::new(&mut output, dimensions, output_stride).unwrap(),
    );
    assert_eq!(
        output,
        [255, 0, 0, 0, 10, 10, 10, 64, 238, 238, 0, 255, 0, 128, 0, 0, 255, 255, 238, 238]
    );
    assert_eq!(&colors[6..8], [-999.0; 2]);
    assert_eq!(&colors[14..16], [-999.0; 2]);
}

#[test]
fn all_perceptual_image_adapters_honor_strides_and_copy_independent_byte_alpha() {
    check_image_round_trip::<Oklab32>(rgba8_to_oklab32_into, oklab32_to_rgba8_into);
    check_image_round_trip::<Oklch32>(rgba8_to_oklch32_into, oklch32_to_rgba8_into);
    check_image_round_trip::<Cielab32>(rgba8_to_cielab32_into, cielab32_to_rgba8_into);
    check_image_round_trip::<Cielch32>(rgba8_to_cielch32_into, cielch32_to_rgba8_into);
}
