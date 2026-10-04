use ditherette_wasm::{
    image::contracts::PaletteEntry,
    spec::{
        contract::{
            error::ErrorCode,
            request::{
                AlphaPolicy, Diffusion, DiffusionFeedback, DitherPolicy, DitherQuantizeRequest,
                MatchPolicy, Placement, QuantizeRequest, Source, WorkingSpace,
            },
        },
        dither::error_diffusion::{
            self as v1, ATKINSON_TAPS, FLOYD_STEINBERG_TAPS, SIERRA_LITE_TAPS, SIERRA_TAPS,
        },
        dither_modes::{
            diffusion::{diffuse, DiffusionPolicy, Kernel},
            ordered::{dither_and_quantize, perturb, tile_noise_at, Tile, TilePerturbPolicy},
        },
        quantize::quantize,
    },
};

const BW: [PaletteEntry; 2] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
];
const TILES: [Tile; 4] = [
    Tile::ThreeByOne,
    Tile::FourByOne,
    Tile::FourByTwo,
    Tile::FiveByThree,
];

fn grays(values: &[u8]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&value| [value, value, value, 255])
        .collect()
}

fn quantize_request(source: &[u8], width: u32, height: u32) -> QuantizeRequest<'_> {
    QuantizeRequest {
        version: 1,
        source: Source {
            width,
            height,
            data: source,
        },
        palette: &BW,
        alpha: AlphaPolicy::Preserve { threshold: 0.0 },
        matching: MatchPolicy::SrgbEuclidean,
    }
}

fn policy(kernel: Kernel) -> DiffusionPolicy {
    DiffusionPolicy {
        kernel,
        strength: 1.0,
        placement: Placement::Everywhere {},
        serpentine: false,
        feedback: DiffusionFeedback::SrgbBytes,
    }
}

#[test]
fn new_kernel_taps_match_their_published_numerators() {
    // Numerators and denominators come from the cited tables, not from the reference constants.
    let cases: &[(Kernel, &[(i32, u32, u32)], u32)] = &[
        (
            Kernel::JarvisJudiceNinke,
            &[
                (1, 0, 7),
                (2, 0, 5),
                (-2, 1, 3),
                (-1, 1, 5),
                (0, 1, 7),
                (1, 1, 5),
                (2, 1, 3),
                (-2, 2, 1),
                (-1, 2, 3),
                (0, 2, 5),
                (1, 2, 3),
                (2, 2, 1),
            ],
            48,
        ),
        (
            Kernel::Stucki,
            &[
                (1, 0, 8),
                (2, 0, 4),
                (-2, 1, 2),
                (-1, 1, 4),
                (0, 1, 8),
                (1, 1, 4),
                (2, 1, 2),
                (-2, 2, 1),
                (-1, 2, 2),
                (0, 2, 4),
                (1, 2, 2),
                (2, 2, 1),
            ],
            42,
        ),
        (
            Kernel::Burkes,
            &[
                (1, 0, 8),
                (2, 0, 4),
                (-2, 1, 2),
                (-1, 1, 4),
                (0, 1, 8),
                (1, 1, 4),
                (2, 1, 2),
            ],
            32,
        ),
        (
            Kernel::TwoRowSierra,
            &[
                (1, 0, 4),
                (2, 0, 3),
                (-2, 1, 1),
                (-1, 1, 2),
                (0, 1, 3),
                (1, 1, 2),
                (2, 1, 1),
            ],
            16,
        ),
        (
            Kernel::Fan,
            &[(1, 0, 7), (-2, 1, 1), (-1, 1, 3), (0, 1, 5)],
            16,
        ),
        (
            Kernel::ShiauFan,
            &[(1, 0, 4), (-2, 1, 1), (-1, 1, 1), (0, 1, 2)],
            8,
        ),
        (
            Kernel::ShiauFan2,
            &[(1, 0, 8), (-3, 1, 1), (-2, 1, 1), (-1, 1, 2), (0, 1, 4)],
            16,
        ),
        (Kernel::Simple2d, &[(1, 0, 1), (0, 1, 1)], 2),
    ];
    for (kernel, positions, denominator) in cases {
        let taps = kernel.taps();
        assert_eq!(taps.len(), positions.len(), "{kernel:?}");
        for (tap, &(dx, dy, numerator)) in taps.iter().zip(*positions) {
            assert_eq!((tap.dx, tap.dy), (dx, dy), "{kernel:?}");
            assert_eq!(tap.weight, numerator as f32 / *denominator as f32);
            assert!(dy > 0 || dx > 0, "every tap follows the raster scan");
            assert!(dy < 3, "every tap fits a three-row ring");
        }
        let total: u32 = positions.iter().map(|&(_, _, numerator)| numerator).sum();
        assert_eq!(total, *denominator, "{kernel:?} distributes all error");
    }
}

#[test]
fn v1_kernels_keep_their_tables_and_v1_results() {
    assert_eq!(Kernel::FloydSteinberg.taps(), FLOYD_STEINBERG_TAPS);
    assert_eq!(Kernel::Sierra.taps(), SIERRA_TAPS);
    assert_eq!(Kernel::SierraLite.taps(), SIERRA_LITE_TAPS);
    assert_eq!(Kernel::Atkinson.taps(), ATKINSON_TAPS);
    let palette = [
        BW[0],
        PaletteEntry::Color { rgb: [181, 31, 91] },
        BW[1],
        PaletteEntry::Transparent {},
    ];
    let (width, height) = (5, 4);
    let data: Vec<u8> = (0..width * height)
        .flat_map(|i| {
            [
                (i * 73 + 17) as u8,
                (i * 31 + 99) as u8,
                (i * 117 + 41) as u8,
                [0, 127, 128, 255][i as usize % 4],
            ]
        })
        .collect();
    for (kernel, v1_kernel) in [
        (Kernel::FloydSteinberg, Diffusion::FloydSteinberg),
        (Kernel::Sierra, Diffusion::Sierra),
        (Kernel::SierraLite, Diffusion::SierraLite),
        (Kernel::Atkinson, Diffusion::Atkinson),
    ] {
        for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
            for matching in [MatchPolicy::SrgbEuclidean, MatchPolicy::OklchHueArc] {
                for serpentine in [false, true] {
                    for placement in [
                        Placement::Everywhere {},
                        Placement::Adaptive {
                            radius: 1,
                            threshold: 5.0,
                            softness: 10.0,
                        },
                    ] {
                        let quantize = QuantizeRequest {
                            palette: &palette,
                            matching,
                            ..quantize_request(&data, width, height)
                        };
                        let extended = DiffusionPolicy {
                            kernel,
                            strength: 0.75,
                            placement,
                            serpentine,
                            feedback,
                        };
                        let expected = v1::diffuse(DitherQuantizeRequest {
                            quantize,
                            dither: DitherPolicy::Diffusion {
                                kernel: v1_kernel,
                                strength: 0.75,
                                placement,
                                serpentine,
                                feedback,
                            },
                        });
                        assert_eq!(diffuse(quantize, extended), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn one_column_reads_each_kernels_downward_taps() {
    let source = grays(&[64, 0, 120]);
    for (kernel, expected) in [
        (Kernel::JarvisJudiceNinke, [0, 0, 1]),
        (Kernel::Stucki, [0, 0, 1]),
        (Kernel::Burkes, [0, 0, 0]),
        (Kernel::TwoRowSierra, [0, 0, 0]),
        (Kernel::Fan, [0, 0, 0]),
        (Kernel::ShiauFan, [0, 0, 0]),
        (Kernel::ShiauFan2, [0, 0, 0]),
        (Kernel::Simple2d, [0, 0, 1]),
    ] {
        // Last unrounded byte: 127.979 / 128.381 / 124 / 122.25 / 126.25 / 124 / 124 / 136.
        assert_eq!(
            diffuse(quantize_request(&source, 1, 3), policy(kernel))
                .unwrap()
                .indices
                .data(),
            expected,
            "{kernel:?}"
        );
    }
}

#[test]
fn shiau_fan_2_alone_reaches_three_pixels_back() {
    // (3,0) emits 64. Only Shiau-Fan 2's (-3,1) tap lifts (0,1) from 125 to 129.
    let source = grays(&[0, 0, 0, 64, 125, 0, 0, 0]);
    for kernel in [
        Kernel::JarvisJudiceNinke,
        Kernel::Stucki,
        Kernel::Burkes,
        Kernel::TwoRowSierra,
        Kernel::Fan,
        Kernel::ShiauFan,
        Kernel::ShiauFan2,
        Kernel::Simple2d,
    ] {
        let first = diffuse(quantize_request(&source, 4, 2), policy(kernel))
            .unwrap()
            .indices
            .data()[4];
        assert_eq!(first, u8::from(kernel == Kernel::ShiauFan2), "{kernel:?}");
    }
}

#[test]
fn kernels_validate_like_v1_and_keep_tags() {
    let source = grays(&[100]);
    let mut invalid = policy(Kernel::Stucki);
    invalid.strength = -1.0;
    let error = diffuse(quantize_request(&source, 1, 1), invalid).unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidSettings, "dither.strength")
    );
    let tags: Vec<String> = [
        Kernel::JarvisJudiceNinke,
        Kernel::TwoRowSierra,
        Kernel::ShiauFan2,
        Kernel::Simple2d,
        Kernel::SierraLite,
    ]
    .iter()
    .map(|kernel| serde_json::to_string(kernel).unwrap())
    .collect();
    assert_eq!(
        tags,
        [
            "\"jarvis-judice-ninke\"",
            "\"two-row-sierra\"",
            "\"shiau-fan-2\"",
            "\"simple-2d\"",
            "\"sierra-lite\""
        ]
    );
}

/// Joel Yliluoma's bit-interleaving generator for power-of-two tiles, ported from his appendix.
fn yliluoma_rank(x: u32, y: u32, m: u32, l: u32) -> u32 {
    let (mut v, mut offset, mut xmask, mut ymask, mut bit) = (0, 0, m, l, 0);
    if m == 0 || (m > l && l != 0) {
        let (xc, yc) = (x ^ ((y << m) >> l), y);
        while bit < m + l {
            ymask -= 1;
            v |= ((yc >> ymask) & 1) << bit;
            bit += 1;
            offset += m;
            while offset >= l {
                xmask -= 1;
                v |= ((xc >> xmask) & 1) << bit;
                bit += 1;
                offset -= l;
            }
        }
    } else {
        let (xc, yc) = (x, y ^ ((x << l) >> m));
        while bit < m + l {
            xmask -= 1;
            v |= ((xc >> xmask) & 1) << bit;
            bit += 1;
            offset += l;
            while offset >= m {
                ymask -= 1;
                v |= ((yc >> ymask) & 1) << bit;
                bit += 1;
                offset -= m;
            }
        }
    }
    v
}

#[test]
fn tiles_are_rank_permutations_from_their_sources() {
    for tile in TILES {
        let mut ranks = tile.ranks().to_vec();
        assert_eq!(ranks.len(), tile.width() * tile.height(), "{tile:?}");
        ranks.sort_unstable();
        assert!(ranks.iter().enumerate().all(|(i, &r)| usize::from(r) == i));
    }
    for (tile, m, l) in [(Tile::FourByOne, 2, 0), (Tile::FourByTwo, 2, 1)] {
        let generated: Vec<u8> = (0..tile.height() as u32)
            .flat_map(|y| (0..tile.width() as u32).map(move |x| yliluoma_rank(x, y, m, l) as u8))
            .collect();
        assert_eq!(tile.ranks(), generated, "{tile:?}");
    }
    // The generator reproduces the appendix's printed 4x4 table too.
    let square: Vec<u32> = (0..4)
        .flat_map(|y| (0..4).map(move |x| yliluoma_rank(x, y, 2, 2)))
        .collect();
    assert_eq!(
        square,
        [0, 12, 3, 15, 8, 4, 11, 7, 2, 14, 1, 13, 10, 6, 9, 5]
    );
    assert_eq!(Tile::ThreeByOne.ranks(), &Tile::FourByOne.ranks()[..3]);
    assert_eq!(
        Tile::FiveByThree.ranks(),
        [0, 12, 7, 3, 9, 14, 8, 1, 5, 11, 6, 4, 10, 13, 2]
    );
    assert_eq!(
        serde_json::to_string(&TILES).unwrap(),
        r#"["3x1","4x1","4x2","5x3"]"#
    );
}

#[test]
fn tile_thresholds_centre_ranks_and_repeat_from_the_image_origin() {
    assert_eq!(tile_noise_at(0, 0, Tile::FourByTwo), 0.5 / 8.0 - 0.5);
    assert_eq!(tile_noise_at(1, 1, Tile::FourByTwo), 7.5 / 8.0 - 0.5);
    assert_eq!(tile_noise_at(5, 3, Tile::FourByTwo), 7.5 / 8.0 - 0.5);
    assert_eq!(
        tile_noise_at(1, 7, Tile::ThreeByOne),
        (2.0f32 + 0.5) / 3.0 - 0.5
    );
    for tile in TILES {
        let (width, height) = (tile.width() as u32, tile.height() as u32);
        let mut sum = 0.0f64;
        for y in 0..height {
            for x in 0..width {
                let value = tile_noise_at(x, y, tile);
                assert!(value > -0.5 && value < 0.5);
                assert_eq!(value, tile_noise_at(x + 2 * width, y + 3 * height, tile));
                sum += f64::from(value);
            }
        }
        assert!(sum.abs() < 1e-6, "{tile:?} mean is zero");
    }
}

#[test]
fn tile_dither_is_perturb_then_v1_quantize() {
    let data: Vec<u8> = (0..30u32)
        .flat_map(|i| [(i * 9) as u8, (i * 5 + 40) as u8, (i * 3 + 80) as u8, 255])
        .collect();
    for tile in TILES {
        let policy = TilePerturbPolicy {
            tile,
            space: WorkingSpace::Oklab,
            strength: 1.0,
            placement: Placement::Everywhere {},
        };
        let request = quantize_request(&data, 6, 5);
        let perturbed = perturb(request.source, policy).unwrap();
        assert_ne!(perturbed.data(), data.as_slice(), "{tile:?} perturbs");
        let expected = quantize(QuantizeRequest {
            source: Source {
                width: 6,
                height: 5,
                data: perturbed.data(),
            },
            ..request
        });
        assert_eq!(dither_and_quantize(request, policy), expected);
    }
    let invalid = TilePerturbPolicy {
        tile: Tile::FourByOne,
        space: WorkingSpace::Srgb,
        strength: -1.0,
        placement: Placement::Everywhere {},
    };
    let error = dither_and_quantize(quantize_request(&data, 6, 5), invalid).unwrap_err();
    assert_eq!(
        (error.code, error.path.as_str()),
        (ErrorCode::InvalidSettings, "dither.perturb.strength")
    );
}
