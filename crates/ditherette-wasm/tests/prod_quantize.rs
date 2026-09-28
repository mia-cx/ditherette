use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions, ImageView, Rgba8},
    prod::{
        self,
        color::{
            packed::{Converter, OrdinarySpace},
            ColorSpaceF32,
        },
        contract::request::{AlphaPolicy, MatchPolicy, QuantizeRequest, Source},
    },
    spec,
};

#[test]
fn diffusion_hue_scores_match_frozen_bits() {
    let check = |a, b| {
        assert_eq!(
            prod::quantize::metric::hue_arc3_squared(a, b).to_bits(),
            spec::quantize::metric::hue_arc3_squared(a, b).to_bits(),
            "{a:?}, {b:?}"
        );
    };
    let tau = std::f32::consts::TAU;
    let mut hues = vec![
        0.0,
        -0.0,
        f32::from_bits(1),
        -0.25,
        -17.0 * tau,
        f32::MAX,
        -f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];
    for turns in [1.0, 2.0, 3.0, 4.0] {
        let bits = (turns * tau).to_bits();
        for offset in -2_i32..=2 {
            let hue = f32::from_bits(bits.wrapping_add_signed(offset));
            hues.extend([hue, -hue]);
        }
    }
    for &left in &hues {
        for &right in &hues {
            check([0.5, 0.25, left], [0.75, 0.1, right]);
            check([-1.0, -0.5, left], [2.0, 0.0, right]);
        }
    }
    let mut bits = 0x87a9_4cd1u32;
    let mut next = || {
        bits ^= bits << 13;
        bits ^= bits >> 17;
        bits ^= bits << 5;
        f32::from_bits(bits)
    };
    for n in 0..100_000 {
        check([0.25, -0.1, next()], [0.75, 0.2, next()]);
        check([0.25, -0.1, n as f32 * 0.001 - 50.0], [0.75, 0.2, next()]);
    }
}

const MODES: [(
    MatchPolicy,
    OrdinarySpace,
    ColorSpaceF32,
    spec::contract::request::WorkingSpace,
); 5] = [
    (
        MatchPolicy::SrgbEuclidean,
        OrdinarySpace::Srgb,
        ColorSpaceF32::Srgb,
        spec::contract::request::WorkingSpace::Srgb,
    ),
    (
        MatchPolicy::LinearRgbEuclidean,
        OrdinarySpace::LinearRgb,
        ColorSpaceF32::LinearSrgb,
        spec::contract::request::WorkingSpace::LinearRgb,
    ),
    (
        MatchPolicy::OklabEuclidean,
        OrdinarySpace::Oklab,
        ColorSpaceF32::Oklab,
        spec::contract::request::WorkingSpace::Oklab,
    ),
    (
        MatchPolicy::CielabEuclidean,
        OrdinarySpace::Cielab,
        ColorSpaceF32::Cielab,
        spec::contract::request::WorkingSpace::Cielab,
    ),
    (
        MatchPolicy::YcbcrEuclidean,
        OrdinarySpace::Ycbcr,
        ColorSpaceF32::YCbCr,
        spec::contract::request::WorkingSpace::Ycbcr,
    ),
];

fn request<'a>(
    data: &'a [u8],
    palette: &'a [PaletteEntry],
    matching: MatchPolicy,
    alpha: AlphaPolicy,
) -> QuantizeRequest<'a> {
    QuantizeRequest {
        version: 1,
        source: Source {
            width: (data.len() / 4) as u32,
            height: 1,
            data,
        },
        palette,
        alpha,
        matching,
    }
}

fn oracle(request: QuantizeRequest<'_>) -> ditherette_wasm::image::contracts::IndexedImage {
    spec::quantize::quantize(spec::contract::request::QuantizeRequest {
        version: request.version,
        source: spec::contract::request::Source {
            width: request.source.width,
            height: request.source.height,
            data: request.source.data,
        },
        palette: request.palette,
        alpha: serde_json::from_value(serde_json::to_value(request.alpha).unwrap()).unwrap(),
        matching: serde_json::from_value(serde_json::to_value(request.matching).unwrap()).unwrap(),
    })
    .unwrap()
}

#[test]
fn packed_triplets_match_frozen_bits_and_preserve_landed_four_channel_api() {
    let data: Vec<u8> = (0..4096u32)
        .flat_map(|n| {
            [
                (n * 73) as u8,
                (n * 31 + n / 256) as u8,
                (n * 17 + 113) as u8,
                n as u8,
            ]
        })
        .collect();
    let original = data.clone();
    let source = ImageView::<Rgba8>::packed(&data, ImageDimensions::new(256, 16).unwrap()).unwrap();
    for (_, space, legacy_space, reference_space) in MODES {
        let converter = Converter::new(space);
        let mut packed = vec![0.0; 4096 * 3];
        converter.rgba8_into(source, &mut packed);
        let legacy = prod::color::rgba8_to_color_space_f32(source, legacy_space, false);
        for ((rgb, actual), old) in data
            .chunks_exact(4)
            .zip(packed.chunks_exact(3))
            .zip(legacy.chunks_exact(4))
        {
            let expected =
                spec::color::rgb8_to_coordinates([rgb[0], rgb[1], rgb[2]], reference_space);
            assert_eq!(
                actual.iter().copied().map(f32::to_bits).collect::<Vec<_>>(),
                expected.map(f32::to_bits)
            );
            assert_eq!(actual, &old[..3]);
            assert_eq!(old[3], f32::from(rgb[3]) / 255.0);
        }
    }
    assert_eq!(data, original);
}

#[test]
fn all_matching_modes_match_indices_palette_alpha_and_warning_bytes() {
    let source: Vec<u8> = (0..256u32)
        .flat_map(|n| [n as u8, (n * 73) as u8, (255 - n) as u8, n as u8])
        .collect();
    let original = source.clone();
    let color = |rgb| PaletteEntry::Color { rgb };
    let mut oversized = vec![color([42, 73, 91]); 257];
    oversized[255] = PaletteEntry::Transparent {};
    let palettes = [
        vec![PaletteEntry::Transparent {}],
        vec![color([0; 3]), color([255; 3])],
        vec![
            PaletteEntry::Transparent {},
            color([0; 3]),
            color([255, 0, 0]),
            color([255, 0, 0]),
            color([255; 3]),
            PaletteEntry::Transparent {},
        ],
        oversized[..256].to_vec(),
        oversized,
    ];
    for matching in all_matching() {
        for palette in &palettes {
            for alpha in [
                AlphaPolicy::Preserve { threshold: 0.0 },
                AlphaPolicy::Preserve {
                    threshold: 127.9999999,
                },
                AlphaPolicy::Preserve { threshold: 255.0 },
                AlphaPolicy::Matte { rgb: [1, 73, 255] },
                AlphaPolicy::Premultiplied {},
            ] {
                let request = request(&source, palette, matching, alpha);
                assert_eq!(
                    prod::quantize::quantize(request, u64::MAX).unwrap(),
                    oracle(request)
                );
            }
        }
    }
    assert_eq!(source, original);
}

#[test]
fn exact_ties_and_transparent_threshold_keep_original_indices() {
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [2, 0, 0] },
    ];
    let source = [1, 0, 0, 255, 2, 0, 0, 128, 2, 0, 0, 127];
    let result = prod::quantize::quantize(
        request(
            &source,
            &palette,
            MatchPolicy::SrgbEuclidean,
            AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
        ),
        u64::MAX,
    )
    .unwrap();
    assert_eq!(result.indices.data(), [1, 2, 0]);
    assert_eq!(result.palette.transparent_index, Some(0));
    assert!(result.warnings.is_empty());
}

#[test]
fn every_metric_retains_duplicate_ties_and_exact_preparation_budget() {
    let palette = [PaletteEntry::Color { rgb: [73; 3] }; 2];
    let alpha = AlphaPolicy::Premultiplied {};
    for matching in all_matching() {
        let capacity =
            prod::quantize::PreparedQuantizer::required_capacity_bytes(&palette, alpha, matching)
                .unwrap();
        let prepared =
            prod::quantize::PreparedQuantizer::try_new(&palette, alpha, matching, capacity)
                .unwrap();
        assert_eq!(prepared.capacity_bytes(), capacity);
        assert!(prod::quantize::PreparedQuantizer::try_new(
            &palette,
            alpha,
            matching,
            capacity - 1
        )
        .is_err());
        assert_eq!(
            prod::quantize::quantize(
                request(&[73, 73, 73, 255], &palette, matching, alpha),
                u64::MAX
            )
            .unwrap()
            .indices
            .data(),
            [0]
        );
    }
}

fn all_matching() -> [MatchPolicy; 15] {
    use MatchPolicy::*;
    [
        SrgbEuclidean,
        LinearRgbEuclidean,
        OklabEuclidean,
        CielabEuclidean,
        YcbcrEuclidean,
        SrgbCompuphase,
        SrgbRec601,
        SrgbRec709,
        OklchEuclidean,
        OklchCircularHue,
        OklchHueArc,
        CielabCiede2000,
        CielchEuclidean,
        CielchCircularHue,
        CielchHueArc,
    ]
}

#[test]
fn packed_cylindrical_conversion_retains_frozen_neutral_and_hue_bits() {
    for (space, reference) in [
        (
            OrdinarySpace::Oklch,
            spec::contract::request::WorkingSpace::Oklch,
        ),
        (
            OrdinarySpace::Cielch,
            spec::contract::request::WorkingSpace::Cielch,
        ),
    ] {
        let converter = Converter::new(space);
        for n in 0..4096u32 {
            let rgb = if n < 256 {
                [n as u8; 3]
            } else {
                [(n * 73) as u8, (n * 31 + n / 256) as u8, (n * 17) as u8]
            };
            let actual = converter.coordinates(rgb);
            assert_eq!(
                actual.map(f32::to_bits),
                spec::color::rgb8_to_coordinates(rgb, reference).map(f32::to_bits)
            );
            assert!(actual[2] >= 0.0 && actual[2] < std::f32::consts::TAU);
            if n < 256 {
                assert_eq!([actual[1].to_bits(), actual[2].to_bits()], [0, 0]);
            }
        }
    }
}

#[test]
fn specialized_scores_match_known_vectors_and_frozen_f32_bits() {
    use prod::quantize::metric::distance_score;
    for (a, b, expected) in [
        ([50.0, 2.6772, -79.7751], [50.0, 0.0, -82.7485], 2.0425),
        ([50.0, 0.0, 0.0], [50.0, -1.0, 2.0], 2.3669),
        ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0009], 7.1792),
        ([50.0, 2.49, -0.001], [50.0, -2.49, 0.0011], 7.2195),
    ] {
        let actual = distance_score(a, b, MatchPolicy::CielabCiede2000);
        assert!((actual - expected).abs() < 0.0001);
    }
    for matching in all_matching() {
        let reference = serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap();
        for (a, b) in [
            ([0.5, 0.0, 0.0], [0.5, 0.25, 3.0]),
            ([0.5, 0.2, 0.05], [0.5, 0.1, std::f32::consts::TAU - 0.05]),
            ([0.5, 0.1, 0.0], [0.5, 0.2, std::f32::consts::FRAC_PI_2]),
        ] {
            assert_eq!(
                distance_score(a, b, matching).to_bits(),
                spec::quantize::metric::distance_score(a, b, reference).to_bits()
            );
        }
    }
    let a = [0.5, 0.1, 0.0];
    let b = [0.5, 0.2, std::f32::consts::FRAC_PI_2];
    assert_ne!(
        distance_score(a, b, MatchPolicy::OklchCircularHue),
        distance_score(a, b, MatchPolicy::OklchHueArc)
    );
}

#[test]
fn reused_scratch_matches_all_metrics_alpha_and_palette_changes() {
    let dimensions = ImageDimensions::new(64, 32).unwrap();
    let source: Vec<u8> = (0..2048u32)
        .flat_map(|n| {
            let color = n % 1024;
            [
                (color * 73) as u8,
                (color * 31 + color / 256) as u8,
                (color * 17) as u8,
                (n / 8) as u8,
            ]
        })
        .collect();
    let view = ImageView::<Rgba8>::packed(&source, dimensions).unwrap();
    let color = |rgb| PaletteEntry::Color { rgb };
    let palettes = [
        vec![
            PaletteEntry::Transparent {},
            color([0; 3]),
            color([2, 0, 0]),
            color([255; 3]),
            color([255; 3]),
        ],
        vec![color([255; 3]), color([0; 3]), color([73; 3])],
        vec![PaletteEntry::Transparent {}],
    ];
    // Deliberately undersized so high-entropy source colors must collide.
    let mut entries = [u64::MAX; 128];
    let mut output = vec![0; 2048];
    for matching in all_matching() {
        for palette in &palettes {
            for alpha in [
                AlphaPolicy::Preserve {
                    threshold: 127.9999999,
                },
                AlphaPolicy::Matte { rgb: [17, 73, 211] },
                AlphaPolicy::Premultiplied {},
            ] {
                let prepared =
                    prod::quantize::PreparedQuantizer::try_new(palette, alpha, matching, u64::MAX)
                        .unwrap();
                let expected = spec::quantize::quantize(spec::contract::request::QuantizeRequest {
                    version: 1,
                    source: spec::contract::request::Source {
                        width: 64,
                        height: 32,
                        data: &source,
                    },
                    palette,
                    alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap()).unwrap(),
                    matching: serde_json::from_value(serde_json::to_value(matching).unwrap())
                        .unwrap(),
                })
                .unwrap();
                let mut completed = 0;
                prepared
                    .quantize_cached_with_progress(view, &mut output, &mut entries, |row| {
                        assert_eq!(row, completed + 1);
                        completed = row;
                        Ok(())
                    })
                    .unwrap();
                assert_eq!(
                    output,
                    expected.indices.data(),
                    "{matching:?}, {alpha:?}, {palette:?}"
                );
                assert_eq!(completed, 32);
                prepared
                    .quantize_cached_with_progress(view, &mut output, &mut [], |_| Ok(()))
                    .unwrap();
                assert_eq!(output, expected.indices.data());
            }
        }
    }
}

#[test]
fn specialized_scans_match_oracle_for_fractional_diffusion_coordinates() {
    use prod::quantize::matcher::{PaletteColor, PaletteMatcher};
    for matching in all_matching() {
        let reference = serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap();
        let converter = Converter::new(OrdinarySpace::from_matching(matching).unwrap());
        let colors: Vec<_> = (0..64u32)
            .map(|n| {
                PaletteColor::new(
                    n as u8,
                    converter.coordinates([(n * 73) as u8, (n * 31) as u8, (n * 17) as u8]),
                )
            })
            .collect();
        let matcher = PaletteMatcher::new(colors, matching);
        for n in 0..256u32 {
            let mut coordinates = converter.coordinates([n as u8, (n * 73) as u8, (n * 17) as u8]);
            coordinates[0] += 0.000123;
            coordinates[1] -= 0.000321;
            let mut expected = matcher.colors()[0];
            let mut best = spec::quantize::metric::distance_score(
                coordinates,
                expected.coordinates,
                reference,
            );
            for &candidate in &matcher.colors()[1..] {
                let score = spec::quantize::metric::distance_score(
                    coordinates,
                    candidate.coordinates,
                    reference,
                );
                if score < best {
                    expected = candidate;
                    best = score;
                }
            }
            assert_eq!(
                matcher.nearest(coordinates),
                expected,
                "{matching:?}, sample {n}"
            );
        }
    }
}

#[test]
fn ciede2000_near_tie_uses_the_complete_frozen_score() {
    use prod::quantize::matcher::{PaletteColor, PaletteMatcher};
    use prod::quantize::metric::distance_score;
    let source = [6.089_999_7, -3.160_780_4, -30.602_425];
    let first = [16.49, 34.222_305, 22.865_76];
    let second = [16.49, 34.222_305, 22.865_799];
    let first_score = distance_score(source, first, MatchPolicy::CielabCiede2000);
    let second_score = distance_score(source, second, MatchPolicy::CielabCiede2000);
    assert!(second_score < first_score);
    assert!(first_score - second_score < first_score * 1e-6);
    assert_eq!(
        [first_score.to_bits(), second_score.to_bits()],
        [
            spec::color::lab_ciede2000::ciede2000(source, first).to_bits(),
            spec::color::lab_ciede2000::ciede2000(source, second).to_bits(),
        ]
    );
    let matcher = PaletteMatcher::new(
        vec![PaletteColor::new(0, first), PaletteColor::new(1, second)],
        MatchPolicy::CielabCiede2000,
    );
    assert_eq!(matcher.nearest(source).index, 1);
}

#[test]
fn ciede2000_distance_matches_frozen_for_rgb_and_near_greys() {
    use prod::quantize::metric::distance_score;
    let converter = Converter::new(OrdinarySpace::Cielab);
    let mut bits = 0x53c9_1e27u32;
    let mut next = || {
        bits ^= bits << 13;
        bits ^= bits >> 17;
        bits ^= bits << 5;
        bits
    };
    for sample in 0..100_000u32 {
        let rgb = |value: u32| [value as u8, (value >> 8) as u8, (value >> 16) as u8];
        let source_rgb = if sample % 8 == 0 {
            let grey = next() as u8;
            [grey, grey, grey.wrapping_add((sample % 3) as u8)]
        } else {
            rgb(next())
        };
        let candidate_rgb = if sample % 11 == 0 {
            let grey = next() as u8;
            [grey, grey.wrapping_add((sample % 2) as u8), grey]
        } else {
            rgb(next())
        };
        let source = converter.coordinates(source_rgb);
        let candidate = converter.coordinates(candidate_rgb);
        assert_eq!(
            distance_score(source, candidate, MatchPolicy::CielabCiede2000).to_bits(),
            spec::color::lab_ciede2000::ciede2000(source, candidate).to_bits(),
            "{source_rgb:?}, {candidate_rgb:?}"
        );
    }
}

#[test]
fn ciede2000_pruning_matches_frozen_for_rgb_and_near_greys() {
    use prod::quantize::matcher::{PaletteColor, PaletteMatcher};
    let converter = Converter::new(OrdinarySpace::Cielab);
    let mut bits = 0x8f31_a2c5u32;
    let mut next = || {
        bits ^= bits << 13;
        bits ^= bits >> 17;
        bits ^= bits << 5;
        bits
    };
    let mut colors = Vec::with_capacity(66);
    for index in 0..64 {
        colors.push(PaletteColor::new(
            index,
            converter.coordinates([next() as u8, (next() >> 8) as u8, (next() >> 16) as u8]),
        ));
    }
    colors.push(PaletteColor::new(64, colors[7].coordinates));
    colors.push(PaletteColor::new(
        65,
        converter.coordinates([128, 128, 129]),
    ));
    let matcher = PaletteMatcher::new(colors, MatchPolicy::CielabCiede2000);

    let expected = |coordinates: [f32; 3], matcher: &PaletteMatcher| {
        let mut best = matcher.colors()[0];
        let mut best_score = spec::color::lab_ciede2000::ciede2000(coordinates, best.coordinates);
        for &candidate in &matcher.colors()[1..] {
            let score = spec::color::lab_ciede2000::ciede2000(coordinates, candidate.coordinates);
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        best
    };
    for n in 0..20_000u32 {
        let rgb = if n % 4 == 0 {
            let gray = next() as u8;
            [gray, gray.wrapping_add((n % 3) as u8), gray]
        } else {
            [next() as u8, (next() >> 8) as u8, (next() >> 16) as u8]
        };
        let coordinates = converter.coordinates(rgb);
        assert_eq!(
            matcher.nearest(coordinates),
            expected(coordinates, &matcher)
        );
    }
    assert_eq!(matcher.nearest(matcher.colors()[7].coordinates).index, 7);

    for palette_case in 0..32 {
        let length = 2 + (next() as usize % 63);
        let mut colors = Vec::with_capacity(length + 1);
        for index in 0..length {
            colors.push(PaletteColor::new(
                index as u8,
                converter.coordinates([next() as u8, (next() >> 8) as u8, (next() >> 16) as u8]),
            ));
        }
        colors[1] = PaletteColor::new(1, colors[0].coordinates);
        colors.push(PaletteColor::new(
            length as u8,
            converter.coordinates([127, 128, 127]),
        ));
        let matcher = PaletteMatcher::new(colors, MatchPolicy::CielabCiede2000);
        for sample in 0..256 {
            let rgb = if sample % 4 == 0 {
                let gray = next() as u8;
                [gray, gray, gray.wrapping_add((sample % 3) as u8)]
            } else {
                [next() as u8, (next() >> 8) as u8, (next() >> 16) as u8]
            };
            let coordinates = converter.coordinates(rgb);
            assert_eq!(
                matcher.nearest(coordinates),
                expected(coordinates, &matcher),
                "palette {palette_case}, RGB {rgb:?}"
            );
        }
    }
}
