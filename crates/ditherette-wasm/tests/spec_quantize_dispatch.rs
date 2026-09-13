use ditherette_wasm::spec::{
    color::{coordinates_to_rgb8, rgb8_to_coordinates},
    contract::request::{MatchPolicy, WorkingSpace},
    quantize::metric::{ciede2000_distance, distance_score, hue_arc3_squared},
};

fn close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} != {expected}"
    );
}

#[test]
fn color_dispatch_uses_all_seven_coordinate_domains_and_inverse_recipes() {
    let cases = [
        (WorkingSpace::Srgb, [1.0, 0.0, 0.0]),
        (WorkingSpace::LinearRgb, [1.0, 0.0, 0.0]),
        (
            WorkingSpace::Oklab,
            [0.627_955_4, 0.224_863_07, 0.125_846_3],
        ),
        (
            WorkingSpace::Oklch,
            [0.627_955_4, 0.257_683_3, 0.510_227_56],
        ),
        (WorkingSpace::Cielab, [53.240_795, 80.092_46, 67.203_19]),
        (
            WorkingSpace::Cielch,
            [53.240_795, 104.551_765, 0.698_114_45],
        ),
        (WorkingSpace::Ycbcr, [0.299, 0.331_264_1, 1.0]),
    ];
    for (space, red) in cases {
        for (actual, expected) in rgb8_to_coordinates([255, 0, 0], space).into_iter().zip(red) {
            close(actual, expected, 0.000_05);
        }
        assert_eq!(coordinates_to_rgb8(red, space), [255, 0, 0], "{space:?}");
        for rgb in [[0; 3], [255; 3], [10, 11, 128], [200, 100, 50]] {
            assert_eq!(
                coordinates_to_rgb8(rgb8_to_coordinates(rgb, space), space),
                rgb
            );
        }
    }
}

#[test]
fn every_metric_tag_uses_its_known_distance_formula() {
    for policy in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::YcbcrEuclidean,
    ] {
        assert_eq!(distance_score([0.0; 3], [1.0; 3], policy), 3.0);
    }
    for (policy, weights) in [
        (MatchPolicy::SrgbRec601, [0.299, 0.587, 0.114]),
        (MatchPolicy::SrgbRec709, [0.2126, 0.7152, 0.0722]),
        (MatchPolicy::SrgbCompuphase, [2.498_046_9, 4.0, 2.996_093_8]),
    ] {
        for (axis, expected) in weights.into_iter().enumerate() {
            let mut primary = [0.0; 3];
            primary[axis] = 1.0;
            close(
                distance_score([0.0; 3], primary, policy),
                expected,
                0.000_001,
            );
        }
    }
    for policy in [
        MatchPolicy::OklchCircularHue,
        MatchPolicy::CielchCircularHue,
    ] {
        close(
            distance_score(
                [0.5, 0.1, 0.0],
                [0.5, 0.2, std::f32::consts::FRAC_PI_2],
                policy,
            ),
            0.05,
            0.000_001,
        );
    }
    for policy in [MatchPolicy::OklchHueArc, MatchPolicy::CielchHueArc] {
        close(
            distance_score(
                [0.5, 0.1, 0.0],
                [0.5, 0.2, std::f32::consts::FRAC_PI_2],
                policy,
            ),
            0.034_674_01,
            0.000_001,
        );
    }
    close(
        distance_score(
            [50.0, 2.6772, -79.7751],
            [50.0, 0.0, -82.7485],
            MatchPolicy::CielabCiede2000,
        ),
        2.0425,
        0.000_1,
    );
}

#[test]
fn website_hue_arc_wraps_at_the_seam_and_ignores_neutral_hue() {
    close(
        hue_arc3_squared([0.5, 0.2, 0.05], [0.5, 0.1, std::f32::consts::TAU - 0.05]),
        0.0101,
        0.000_001,
    );
    assert_eq!(hue_arc3_squared([0.5, 0.0, 0.0], [0.5, 0.25, 3.0]), 0.0625);
}

#[test]
fn ciede2000_matches_published_signed_hue_neutral_and_boundary_pairs() {
    // Sharma, Wu, Dalal supplementary data. https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/
    // These include both sides of the 180-degree hue discontinuity, not just the original blue example.
    let cases = [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 3.1571, -77.2803], [50.0, 0.0, -82.7485], 2.8615),
        ([50.0, -1.3802, -84.2814], [50.0, 0.0, -82.7485], 1.0),
        ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
        ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0009], 7.1792),
        ([50.0, 2.49, -0.001], [50.0, -2.49, 0.001], 7.1792),
        ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0011], 7.2195),
        ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0012], 7.2195),
        ([50.0, -0.001, 2.49], [50.0, 0.0009, -2.49], 4.8045),
        ([50.0, -0.001, 2.49], [50.0, 0.001, -2.49], 4.8045),
        ([50.0, -0.001, 2.49], [50.0, 0.0011, -2.49], 4.7461),
        ([50.0, 2.5, 0.0], [50.0, 0.0, -2.5], 4.3065),
        ([50.0, 2.5, 0.0], [73.0, 25.0, -18.0], 27.1492),
        ([50.0, 2.5, 0.0], [61.0, -5.0, 29.0], 22.8977),
        ([50.0, 2.5, 0.0], [56.0, -27.0, -3.0], 31.9030),
        (
            [6.7747, -0.2908, -2.4247],
            [5.8714, -0.0985, -2.2286],
            0.6377,
        ),
    ];
    for (a, b, expected) in cases {
        close(ciede2000_distance(a, b), expected, 0.000_1);
        close(ciede2000_distance(b, a), expected, 0.000_1);
        assert_eq!(ciede2000_distance(a, a), 0.0);
    }
}
