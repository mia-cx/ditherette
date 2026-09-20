use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride},
    prod::{
        self,
        contract::request::{
            AlphaPolicy, MatchPolicy, Placement, QuantizeRequest, Source, WorkingSpace,
        },
        tiling::RowBand,
    },
    spec,
};

const SPACES: [WorkingSpace; 7] = [
    WorkingSpace::Srgb,
    WorkingSpace::LinearRgb,
    WorkingSpace::Oklab,
    WorkingSpace::Oklch,
    WorkingSpace::Cielab,
    WorkingSpace::Cielch,
    WorkingSpace::Ycbcr,
];

fn reference_space(space: WorkingSpace) -> spec::contract::request::WorkingSpace {
    serde_json::from_value(serde_json::to_value(space).unwrap()).unwrap()
}

fn reference_placement(placement: Placement) -> spec::contract::request::Placement {
    serde_json::from_value(serde_json::to_value(placement).unwrap()).unwrap()
}

fn bytes(width: u32, height: u32, stride: usize) -> Vec<u8> {
    let mut data = vec![203; stride * height as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let n = y * width as usize + x;
            data[y * stride + x * 4..y * stride + x * 4 + 4].copy_from_slice(&[
                (n * 73) as u8,
                (n * 31 + 127) as u8,
                (n * 17 + 255) as u8,
                [0, 1, 127, 128, 254, 255][n % 6],
            ]);
        }
    }
    data
}

#[test]
fn f32_inverses_and_wide_reconstruction_keep_their_separate_oracles() {
    for space in SPACES {
        let oracle = reference_space(space);
        for rgb in [
            [0; 3],
            [128; 3],
            [255; 3],
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [17, 33, 71],
        ] {
            let coordinates = prod::color::packed::rgb8_to_coordinates(rgb, space);
            assert_eq!(
                coordinates.map(f32::to_bits),
                spec::color::rgb8_to_coordinates(rgb, oracle).map(f32::to_bits)
            );
            for value in [
                coordinates,
                [-0.5, 0.5, 1.5],
                [128.0 / 255.0; 3],
                [0.5, -1.0, -std::f32::consts::TAU],
                [100.0, 147.0, 7.0],
            ] {
                assert_eq!(
                    prod::color::inverse::coordinates_to_rgb8(value, space),
                    spec::color::coordinates_to_rgb8(value, oracle)
                );
                for scale in [1.0, f64::from(f32::MAX)] {
                    let wide = value.map(|v| f64::from(v) * scale);
                    assert_eq!(
                        prod::color::reconstruct::coordinates_to_rgb8(wide, space),
                        spec::color::reconstruct::coordinates_to_rgb8(wide, oracle)
                    );
                    let actual = prod::color::reconstruct::coordinates_to_srgb(wide, space);
                    assert!(actual.into_iter().all(f64::is_finite));
                    assert_eq!(
                        actual.map(f64::to_bits),
                        spec::color::reconstruct::coordinates_to_srgb(wide, oracle)
                            .map(f64::to_bits)
                    );
                }
            }
        }
    }
}

#[test]
fn inverse_image_adapters_copy_alpha_and_leave_padding_untouched() {
    let dimensions = ImageDimensions::new(3, 2).unwrap();
    let alpha = bytes(3, 2, 16);
    let alpha_view = ImageView::new(&alpha, dimensions, RowStride::new(16).unwrap()).unwrap();
    let coordinates: Vec<f32> = (0..24).map(|n| (n as f32 - 3.0) / 11.0).collect();
    macro_rules! check {
        ($module:ident, $format:ident, $inverse:ident) => {{
            let input = ImageView::<ditherette_wasm::image::$format>::new(
                &coordinates,
                dimensions,
                RowStride::new(12).unwrap(),
            )
            .unwrap();
            let mut actual = vec![211; 40];
            let mut expected = actual.clone();
            prod::color::$module::$inverse(
                input,
                alpha_view,
                ImageViewMut::new(&mut actual, dimensions, RowStride::new(20).unwrap()).unwrap(),
            );
            spec::color::$module::$inverse(
                input,
                alpha_view,
                ImageViewMut::new(&mut expected, dimensions, RowStride::new(20).unwrap()).unwrap(),
            );
            assert_eq!(actual, expected);
            for y in 0..2 {
                assert_eq!(&actual[y * 20 + 12..y * 20 + 20], &[211; 8]);
                for x in 0..3 {
                    assert_eq!(actual[y * 20 + x * 4 + 3], alpha[y * 16 + x * 4 + 3]);
                }
            }
        }};
    }
    check!(srgb, Srgb32, srgb32_to_rgba8_into);
    check!(linear, LinearRgb32, linear_rgb32_to_rgba8_into);
    check!(oklab, Oklab32, oklab32_to_rgba8_into);
    check!(oklch, Oklch32, oklch32_to_rgba8_into);
    check!(cielab, Cielab32, cielab32_to_rgba8_into);
    check!(cielch, Cielch32, cielch32_to_rgba8_into);
    check!(ycbcr, YCbCr32, ycbcr32_to_rgba8_into);
}

#[test]
fn bayer_and_random_values_keep_global_coordinate_and_wrapping_identity() {
    use prod::dither::ordered::BayerSize as P;
    use spec::dither::ordered::BayerSize as R;
    for (p, r) in [
        (P::Two, R::Two),
        (P::Four, R::Four),
        (P::Eight, R::Eight),
        (P::Sixteen, R::Sixteen),
    ] {
        for y in 0..32 {
            for x in 0..32 {
                assert_eq!(
                    prod::dither::ordered::bayer_value(x, y, p.width()),
                    spec::dither::ordered::bayer_value(x, y, r.width())
                );
                assert_eq!(
                    prod::dither::ordered::bayer_noise_at(x as u32, y as u32, p).to_bits(),
                    spec::dither::ordered::bayer_noise_at(x as u32, y as u32, r).to_bits()
                );
            }
        }
    }
    for seed in [0, 1, 0x1234_5678, u32::MAX] {
        for index in [0, 1, 2, 255, u64::from(u32::MAX), 1_u64 << 32, u64::MAX] {
            assert_eq!(
                prod::dither::random_noise::random_u32_at(seed, index),
                spec::dither::random_noise::random_u32_at(seed, index)
            );
            assert_eq!(
                prod::dither::random_noise::random_noise_at(seed, index).to_bits(),
                spec::dither::random_noise::random_noise_at(seed, index).to_bits()
            );
        }
    }
}

fn field(kind: usize, x: u32, y: u32, index: u64) -> f32 {
    use prod::dither::{
        ordered::{bayer_noise_at, BayerSize},
        random_noise::random_noise_at,
    };
    match kind {
        0 => bayer_noise_at(x, y, BayerSize::Two),
        1 => bayer_noise_at(x, y, BayerSize::Four),
        2 => bayer_noise_at(x, y, BayerSize::Eight),
        3 => bayer_noise_at(x, y, BayerSize::Sixteen),
        _ => random_noise_at(0x1234_5678, index),
    }
}

fn reference_field(kind: usize, x: u32, y: u32, index: u64) -> f32 {
    use spec::dither::{
        ordered::{bayer_noise_at, BayerSize},
        random_noise::random_noise_at,
    };
    match kind {
        0 => bayer_noise_at(x, y, BayerSize::Two),
        1 => bayer_noise_at(x, y, BayerSize::Four),
        2 => bayer_noise_at(x, y, BayerSize::Eight),
        3 => bayer_noise_at(x, y, BayerSize::Sixteen),
        _ => random_noise_at(0x1234_5678, index),
    }
}

#[test]
fn strided_bands_preserve_every_field_space_strength_and_adaptive_byte_boundary() {
    for (width, height) in [(1, 1), (1, 4), (5, 3)] {
        let dimensions = ImageDimensions::new(width, height).unwrap();
        let source_stride = width as usize * 4 + 4;
        let output_stride = width as usize * 4 + 8;
        let source_bytes = bytes(width, height, source_stride);
        let original = source_bytes.clone();
        let source = ImageView::new(
            &source_bytes,
            dimensions,
            RowStride::new(source_stride).unwrap(),
        )
        .unwrap();
        for space in SPACES {
            for kind in 0..5 {
                for strength in [0.0, 0.7, f32::MAX] {
                    for placement in [
                        Placement::Everywhere {},
                        Placement::Adaptive {
                            radius: 1,
                            threshold: 5.0,
                            softness: 3.0,
                        },
                        Placement::Adaptive {
                            radius: 32768,
                            threshold: 100.0,
                            softness: 0.0,
                        },
                    ] {
                        let mut actual = vec![211; output_stride * height as usize];
                        let mut expected = actual.clone();
                        let observed = std::cell::RefCell::new(Vec::new());
                        // Reverse row scheduling still owns the same global draw per pixel.
                        for y in (0..height).rev() {
                            prod::dither::perturb::perturb_by_field_rows_into(
                                source,
                                ImageViewMut::new(
                                    &mut actual,
                                    dimensions,
                                    RowStride::new(output_stride).unwrap(),
                                )
                                .unwrap(),
                                space,
                                strength,
                                placement,
                                RowBand::new(y, y + 1).unwrap(),
                                |x, y, i| {
                                    observed.borrow_mut().push(i);
                                    field(kind, x, y, i)
                                },
                            );
                        }
                        spec::dither::perturb::perturb_by_field_rows_into(
                            source,
                            ImageViewMut::new(
                                &mut expected,
                                dimensions,
                                RowStride::new(output_stride).unwrap(),
                            )
                            .unwrap(),
                            reference_space(space),
                            strength,
                            reference_placement(placement),
                            spec::tiling::contract::RowBand::new(0, height).unwrap(),
                            |x, y, i| reference_field(kind, x, y, i),
                        );
                        assert_eq!(
                            actual, expected,
                            "{space:?}/{kind}/{strength}/{placement:?}"
                        );
                        let mut draws = observed.into_inner();
                        draws.sort_unstable();
                        assert_eq!(
                            draws,
                            (0..u64::from(width) * u64::from(height)).collect::<Vec<_>>()
                        );
                        for y in 0..height as usize {
                            assert_eq!(
                                &actual[y * output_stride + width as usize * 4
                                    ..(y + 1) * output_stride],
                                &[211; 8]
                            );
                            for x in 0..width as usize {
                                assert_eq!(
                                    actual[y * output_stride + x * 4 + 3],
                                    source_bytes[y * source_stride + x * 4 + 3]
                                );
                                if strength == 0.0 {
                                    assert_eq!(
                                        &actual[y * output_stride + x * 4
                                            ..y * output_stride + x * 4 + 4],
                                        &source_bytes[y * source_stride + x * 4
                                            ..y * source_stride + x * 4 + 4]
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(source_bytes, original);
    }
}

#[test]
fn placement_edges_and_smoothstep_match_frozen_bits() {
    let dimensions = ImageDimensions::new(3, 2).unwrap();
    let data = bytes(3, 2, 16);
    let source = ImageView::<Rgba8>::new(&data, dimensions, RowStride::new(16).unwrap()).unwrap();
    for space in SPACES {
        let oracle = reference_space(space);
        assert_eq!(
            prod::dither::placement::coordinate_domain(space).ranges(),
            spec::dither::placement::coordinate_domain(oracle).ranges()
        );
        for y in 0..2 {
            for x in 0..3 {
                for radius in [1, 2, 32768] {
                    let contrast =
                        spec::dither::placement::contrast_at(source, x, y, oracle, radius);
                    assert_eq!(
                        prod::dither::placement::contrast_at(source, x, y, space, radius).to_bits(),
                        contrast.to_bits()
                    );
                    for (threshold, softness) in [(0.0, 0.0), (contrast as f32, 10.0), (100.0, 0.0)]
                    {
                        let placement = Placement::Adaptive {
                            radius,
                            threshold,
                            softness,
                        };
                        assert_eq!(
                            prod::dither::placement::placement_mask_at(
                                source, x, y, space, placement
                            )
                            .to_bits(),
                            spec::dither::placement::placement_mask_at(
                                source,
                                x,
                                y,
                                oracle,
                                reference_placement(placement)
                            )
                            .to_bits()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn quantization_observes_reconstructed_bytes_for_every_matching_policy() {
    let data = bytes(4, 2, 16);
    let dimensions = ImageDimensions::new(4, 2).unwrap();
    let source = ImageView::packed(&data, dimensions).unwrap();
    let policies = [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::YcbcrEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielchHueArc,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::CielabCiede2000,
    ];
    let palettes = [
        vec![PaletteEntry::Transparent {}],
        vec![
            PaletteEntry::Color { rgb: [0; 3] },
            PaletteEntry::Color { rgb: [255; 3] },
            PaletteEntry::Color { rgb: [255; 3] },
            PaletteEntry::Transparent {},
        ],
    ];
    for space in SPACES {
        let mut actual = vec![0; data.len()];
        let mut expected = actual.clone();
        prod::dither::perturb::perturb_by_field_rows_into(
            source,
            ImageViewMut::packed(&mut actual, dimensions).unwrap(),
            space,
            0.7,
            Placement::Everywhere {},
            RowBand::new(0, 2).unwrap(),
            |x, y, i| field(4, x, y, i),
        );
        spec::dither::perturb::perturb_by_field_rows_into(
            source,
            ImageViewMut::packed(&mut expected, dimensions).unwrap(),
            reference_space(space),
            0.7,
            spec::contract::request::Placement::Everywhere {},
            spec::tiling::contract::RowBand::new(0, 2).unwrap(),
            |x, y, i| reference_field(4, x, y, i),
        );
        assert_eq!(actual, expected);
        for matching in policies {
            for palette in &palettes {
                for alpha in [
                    AlphaPolicy::Preserve {
                        threshold: 127.9999999,
                    },
                    AlphaPolicy::Premultiplied {},
                    AlphaPolicy::Matte { rgb: [17, 33, 71] },
                ] {
                    let result = prod::quantize::quantize(
                        QuantizeRequest {
                            version: 1,
                            source: Source {
                                width: 4,
                                height: 2,
                                data: &actual,
                            },
                            palette,
                            matching,
                            alpha,
                        },
                        u64::MAX,
                    )
                    .unwrap();
                    let oracle =
                        spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                            version: 1,
                            source: spec::contract::request::Source {
                                width: 4,
                                height: 2,
                                data: &expected,
                            },
                            palette,
                            matching: serde_json::from_value(
                                serde_json::to_value(matching).unwrap(),
                            )
                            .unwrap(),
                            alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap())
                                .unwrap(),
                        })
                        .unwrap();
                    assert_eq!(result, oracle);
                }
            }
        }
    }
}
