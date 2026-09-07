use ditherette_wasm::spec::{
    color::{
        cielab::rgb8_to_cielab,
        cielch::rgb8_to_cielch,
        linear::rgb8_to_linear_rgb,
        oklab::rgb8_to_oklab,
        oklch::rgb8_to_oklch,
        reconstruct::{coordinates_to_rgb8, coordinates_to_srgb},
        srgb::rgb8_to_srgb,
        ycbcr::rgb8_to_ycbcr,
    },
    contract::request::WorkingSpace,
    dither::placement::coordinate_domain,
};

#[test]
fn wide_reconstruction_round_trips_the_existing_f32_forward_coordinates() {
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
        for r in [0, 1, 10, 11, 64, 128, 254, 255] {
            for g in [0, 1, 10, 11, 64, 128, 254, 255] {
                for b in [0, 1, 10, 11, 64, 128, 254, 255] {
                    let rgb = [r, g, b];
                    assert_eq!(
                        coordinates_to_rgb8(convert(rgb).map(f64::from), space),
                        rgb,
                        "{space:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn wide_reconstruction_retains_known_values_and_byte_rounding() {
    assert_eq!(
        coordinates_to_rgb8([0.5, 0.5 / 255.0, 1.5 / 255.0], WorkingSpace::Srgb),
        [128, 1, 2]
    );
    assert_eq!(
        coordinates_to_rgb8([-1.0, 2.0, 0.0], WorkingSpace::Srgb),
        [0, 255, 0]
    );
    assert_eq!(
        coordinates_to_rgb8([0.18, 0.5, 1.0], WorkingSpace::LinearRgb),
        [118, 188, 255]
    );
    assert_eq!(
        coordinates_to_rgb8([0.5, 0.5, 0.5], WorkingSpace::Ycbcr),
        [128, 128, 128]
    );
    assert_eq!(
        coordinates_to_rgb8([0.62795536, 0.22486306, 0.12584630], WorkingSpace::Oklab),
        [255, 0, 0]
    );
    assert_eq!(
        coordinates_to_rgb8([53.24079, 80.09246, 67.20320], WorkingSpace::Cielab),
        [255, 0, 0]
    );
}

#[test]
fn wide_cylindrical_reconstruction_wraps_hue_and_ignores_it_for_neutral_chroma() {
    for (cylindrical, cartesian, lightness, chroma) in [
        (WorkingSpace::Oklch, WorkingSpace::Oklab, 0.5, 0.1),
        (WorkingSpace::Cielch, WorkingSpace::Cielab, 50.0, 10.0),
    ] {
        let expected = coordinates_to_rgb8([lightness, chroma, 0.0], cylindrical);
        assert_eq!(
            coordinates_to_rgb8([lightness, chroma, std::f64::consts::TAU], cylindrical),
            expected
        );
        assert_eq!(
            coordinates_to_rgb8([lightness, chroma, -std::f64::consts::TAU], cylindrical),
            expected
        );
        assert_eq!(
            coordinates_to_rgb8([lightness, -1.0, 1e40], cylindrical),
            coordinates_to_rgb8([lightness, 0.0, 0.0], cartesian)
        );
    }
}

#[test]
fn every_space_stays_finite_through_inverse_at_maximum_legal_field_strength() {
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        let domain = coordinate_domain(space);
        let widths = domain.ranges().map(f64::from);
        for source in [domain.minimum, domain.maximum] {
            for threshold in [-0.5, 0.5] {
                let coordinates = std::array::from_fn(|axis| {
                    f64::from(source[axis]) + threshold * f64::from(f32::MAX) * 0.25 * widths[axis]
                });
                assert!(
                    coordinates_to_srgb(coordinates, space)
                        .into_iter()
                        .all(f64::is_finite),
                    "{space:?}"
                );
            }
        }
    }
}
