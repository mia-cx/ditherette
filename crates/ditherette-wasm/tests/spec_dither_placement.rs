use ditherette_wasm::spec::{
    color::{
        cielab::rgb8_to_cielab, cielch::rgb8_to_cielch, linear::rgb8_to_linear_rgb,
        oklab::rgb8_to_oklab, oklch::rgb8_to_oklch, srgb::rgb8_to_srgb, ycbcr::rgb8_to_ycbcr,
    },
    contract::request::WorkingSpace,
    dither::placement::coordinate_domain,
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
