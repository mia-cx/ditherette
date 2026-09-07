use ditherette_wasm::spec::color::{
    cielab::{cielab_to_rgb8, rgb8_to_cielab},
    oklab::{oklab_to_rgb8, rgb8_to_oklab},
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
fn cielab_inverse_crosses_the_linear_cubic_threshold_at_lightness_eight() {
    // The inverse neutral Y is L/kappa below L=8, and ((L+16)/116)^3 above it.
    // Independently applying sRGB encoding gives bytes 23, 24, and 25.
    for (lightness, byte) in [(7.5, 23), (8.0, 24), (9.0, 25)] {
        assert_eq!(cielab_to_rgb8([lightness, 0.0, 0.0]), [byte; 3]);
    }
}
