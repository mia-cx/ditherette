use ditherette_wasm::image::{ImageDimensions, ImageView, Rgba8, RowStride};
use ditherette_wasm::spec::{
    color::{
        cielab::rgb8_to_cielab, cielch::rgb8_to_cielch, linear::rgb8_to_linear_rgb,
        oklab::rgb8_to_oklab, oklch::rgb8_to_oklch, srgb::rgb8_to_srgb, ycbcr::rgb8_to_ycbcr,
    },
    contract::request::{Placement, WorkingSpace},
    dither::placement::{contrast_at, coordinate_domain, placement_distance, placement_mask_at},
};

#[test]
fn fixed_domains_enclose_sampled_source_gamuts_and_the_proven_extrema() {
    let conversions: [(WorkingSpace, fn([u8; 3]) -> [f32; 3]); 7] = [
        (WorkingSpace::Srgb, rgb8_to_srgb),
        (WorkingSpace::LinearRgb, rgb8_to_linear_rgb),
        (WorkingSpace::Oklab, rgb8_to_oklab),
        (WorkingSpace::Oklch, rgb8_to_oklch),
        (WorkingSpace::Cielab, rgb8_to_cielab),
        (WorkingSpace::Cielch, rgb8_to_cielch),
        (WorkingSpace::Ycbcr, rgb8_to_ycbcr),
    ];
    for (space, convert) in conversions {
        let domain = coordinate_domain(space);
        for r in [0, 1, 10, 11, 64, 128, 254, 255] {
            for g in [0, 1, 10, 11, 64, 128, 254, 255] {
                for b in [0, 1, 10, 11, 64, 128, 254, 255] {
                    let coordinates = convert([r, g, b]);
                    for (axis, value) in coordinates.into_iter().enumerate() {
                        assert!(
                            value >= domain.minimum[axis] && value <= domain.maximum[axis],
                            "{space:?} [{r}, {g}, {b}]: {coordinates:?}"
                        );
                    }
                }
            }
        }
        assert!(domain
            .ranges()
            .into_iter()
            .all(|width| width.is_finite() && width > 0.0));
    }
}

fn packed(data: &[u8], width: u32, height: u32) -> ImageView<'_, Rgba8> {
    ImageView::packed(data, ImageDimensions::new(width, height).unwrap()).unwrap()
}

fn adaptive(radius: u32, threshold: f32, softness: f32) -> Placement {
    Placement::Adaptive {
        radius,
        threshold,
        softness,
    }
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.00001,
        "{actual} differs from {expected}"
    );
}

#[test]
fn eight_samples_include_diagonals_with_equal_weight() {
    let mut data = [0; 3 * 3 * 4];
    data[..3].fill(255);
    close(
        contrast_at(packed(&data, 3, 3), 1, 1, WorkingSpace::Srgb, 1),
        12.5,
    );
    data.fill(255);
    data[16..19].fill(0);
    close(
        contrast_at(packed(&data, 3, 3), 1, 1, WorkingSpace::Srgb, 1),
        100.0,
    );
}

#[test]
fn border_clamping_counts_repeated_neighbors_and_radius_selects_sample_distance() {
    let data = [0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255];
    let source = packed(&data, 3, 1);
    close(contrast_at(source, 0, 0, WorkingSpace::Srgb, 1), 0.0);
    close(contrast_at(source, 0, 0, WorkingSpace::Srgb, 2), 37.5);
    close(contrast_at(source, 0, 0, WorkingSpace::Srgb, 32768), 37.5);
    close(contrast_at(source, 1, 0, WorkingSpace::Srgb, 1), 37.5);
    close(contrast_at(source, 2, 0, WorkingSpace::Srgb, 1), 37.5);
}

#[test]
fn threshold_softness_and_hard_edge_follow_smoothstep() {
    let data = [0, 0, 0, 255, 255, 255, 255, 255];
    let source = packed(&data, 2, 1); // Contrast is exactly 37.5 percent.
    for (threshold, softness, expected) in [
        (37.5, 0.0, 1.0),
        (37.5001, 0.0, 0.0),
        (37.5, 10.0, 0.5),
        (47.5, 10.0, 0.0),
        (27.5, 10.0, 1.0),
        (42.5, 10.0, 0.15625),
    ] {
        close(
            f64::from(placement_mask_at(
                source,
                0,
                0,
                WorkingSpace::Srgb,
                adaptive(1, threshold, softness),
            )),
            expected,
        );
    }
}

#[test]
fn uniform_images_include_the_zero_threshold_boundary_in_every_space() {
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        for rgb in [[0, 0, 0], [128, 128, 128], [255, 255, 255], [255, 0, 255]] {
            let data = [rgb[0], rgb[1], rgb[2], 0];
            let source = packed(&data, 1, 1);
            assert_eq!(contrast_at(source, 0, 0, space, 32768), 0.0);
            assert_eq!(
                placement_mask_at(source, 0, 0, space, adaptive(1, 0.0, 0.0)),
                1.0
            );
            assert_eq!(
                placement_mask_at(source, 0, 0, space, adaptive(1, 0.1, 0.0)),
                0.0
            );
            assert_eq!(
                placement_mask_at(source, 0, 0, space, Placement::Everywhere {}),
                1.0
            );
        }
    }
}

#[test]
fn cylindrical_distance_uses_minimum_chroma_arc_not_matching_chord() {
    for space in [WorkingSpace::Oklch, WorkingSpace::Cielch] {
        // Chroma mismatch makes minimum-chroma arc differ from the matching chord metric.
        let left = [0.5, 0.2, 0.0];
        let right = [0.5, 0.4, std::f32::consts::FRAC_PI_2];
        close(
            placement_distance(space, left, right),
            (0.2_f64.powi(2) + (0.2 * std::f64::consts::FRAC_PI_2).powi(2)).sqrt(),
        );
        close(
            placement_distance(
                space,
                [0.5, 0.2, 0.1],
                [0.5, 0.2, std::f32::consts::TAU - 0.1],
            ),
            0.04,
        );
        close(placement_distance(space, [0.5, 0.0, 0.0], right), 0.4);
        close(
            placement_distance(space, right, left),
            placement_distance(space, left, right),
        );
    }
    assert_eq!(
        placement_distance(WorkingSpace::Srgb, [0.0; 3], [1.0, 2.0, 2.0]),
        3.0
    );
}

#[test]
fn placement_reads_hidden_rgb_and_ignores_alpha_and_row_padding() {
    let opaque = [0, 0, 0, 255, 255, 255, 255, 255];
    let hidden_strided = [0, 0, 0, 0, 7, 8, 9, 255, 255, 255, 0];
    let dimensions = ImageDimensions::new(1, 2).unwrap();
    let hidden =
        ImageView::<Rgba8>::new(&hidden_strided, dimensions, RowStride::new(7).unwrap()).unwrap();
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        let visible = packed(&opaque, 1, 2);
        assert_eq!(
            contrast_at(hidden, 0, 0, space, 1),
            contrast_at(visible, 0, 0, space, 1)
        );
        assert_eq!(
            placement_mask_at(hidden, 0, 0, space, adaptive(1, 2.0, 5.0)),
            placement_mask_at(visible, 0, 0, space, adaptive(1, 2.0, 5.0))
        );
    }
}

#[test]
fn finite_extreme_controls_never_overflow_the_mask() {
    let data = [0, 0, 0, 0];
    let source = packed(&data, 1, 1);
    assert_eq!(
        placement_mask_at(
            source,
            0,
            0,
            WorkingSpace::Srgb,
            adaptive(1, f32::MAX, f32::MAX)
        ),
        0.0
    );
    assert_eq!(
        placement_mask_at(source, 0, 0, WorkingSpace::Srgb, adaptive(1, 0.0, f32::MAX)),
        0.5
    );
}

#[test]
fn working_space_selects_its_conversion_and_its_fixed_normalization() {
    let data = [0, 0, 0, 255, 128, 128, 128, 255];
    let source = packed(&data, 2, 1);
    close(
        contrast_at(source, 0, 0, WorkingSpace::Srgb, 1),
        37.5 * 128.0 / 255.0,
    );
    close(
        contrast_at(source, 0, 0, WorkingSpace::LinearRgb, 1),
        37.5 * 0.2158605,
    );

    let endpoints = [0, 0, 0, 255, 255, 255, 255, 255];
    let source = packed(&endpoints, 2, 1);
    // Known black/white differences, with tiny f32 matrix residuals below the fixture tolerance.
    for (space, lightness_difference) in [
        (WorkingSpace::Oklab, 1.0),
        (WorkingSpace::Oklch, 1.0),
        (WorkingSpace::Cielab, 100.0),
        (WorkingSpace::Cielch, 100.0),
        (WorkingSpace::Ycbcr, 1.0),
    ] {
        let widths = coordinate_domain(space).ranges().map(f64::from);
        let diagonal = widths
            .into_iter()
            .map(|width| width * width)
            .sum::<f64>()
            .sqrt();
        close(
            contrast_at(source, 0, 0, space, 1),
            37.5 * lightness_difference / diagonal,
        );
    }
}
