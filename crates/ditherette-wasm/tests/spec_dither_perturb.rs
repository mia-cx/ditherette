use ditherette_wasm::spec::dither::{
    ordered::{bayer_noise_at, BayerSize},
    random_noise::{random_noise_at, random_u32_at, Mulberry32},
};

use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageBuf, ImageDimensions, ImageView, Rgba8, RowStride},
    spec::{
        contract::request::{
            AlphaPolicy, BayerSize as RequestBayerSize, Field, PerturbPolicy, Placement,
            WorkingSpace,
        },
        dither::perturb::{
            perturb, perturb_by_field_rows_into, perturb_rows_into, quantize_after_perturb,
        },
        palette::{PalettePixel, PreparedPalette},
        tiling::contract::RowBand,
    },
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

const FIELDS: [Field; 5] = [
    Field::Bayer {
        size: RequestBayerSize::Two,
    },
    Field::Bayer {
        size: RequestBayerSize::Four,
    },
    Field::Bayer {
        size: RequestBayerSize::Eight,
    },
    Field::Bayer {
        size: RequestBayerSize::Sixteen,
    },
    Field::Random { seed: 123 },
];

fn source(data: &[u8], width: u32, height: u32) -> ImageView<'_, Rgba8> {
    ImageView::packed(data, ImageDimensions::new(width, height).unwrap()).unwrap()
}

fn policy(field: Field, space: WorkingSpace, strength: f32) -> PerturbPolicy {
    PerturbPolicy {
        field,
        space,
        strength,
        placement: Placement::Everywhere,
    }
}

#[test]
fn bayer_orientation_matches_the_website_and_all_sizes_cover_centered_ranks() {
    let four = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(
                bayer_noise_at(x, y, BayerSize::Four),
                (four[y as usize][x as usize] as f32 + 0.5) / 16.0 - 0.5
            );
        }
    }
    for size in [
        BayerSize::Two,
        BayerSize::Four,
        BayerSize::Eight,
        BayerSize::Sixteen,
    ] {
        let width = size.width() as u32;
        let mut values = Vec::new();
        for y in 0..width {
            for x in 0..width {
                let value = bayer_noise_at(x, y, size);
                assert_eq!(value, bayer_noise_at(x + width, y + 3 * width, size));
                values.push(value);
            }
        }
        values.sort_by(f32::total_cmp);
        assert_eq!(values.iter().sum::<f32>(), 0.0);
        for (rank, value) in values.into_iter().enumerate() {
            assert_eq!(value, (rank as f32 + 0.5) / (width * width) as f32 - 0.5);
        }
    }
}

#[test]
fn global_random_draws_match_independent_javascript_integer_fixtures() {
    // Independent Math.imul evaluation of the existing website mixer, not Rust-generated snapshots.
    for (seed, expected) in [
        (
            0,
            [
                1144304738, 1416247, 958946056, 2787370982, 1958536065, 2834277798, 3461135945,
            ],
        ),
        (
            1,
            [
                2693262067, 11749833, 2265367787, 3095568220, 1828783984, 663542962, 2100857034,
            ],
        ),
        (
            u32::MAX,
            [
                3850105811, 813802916, 3073704848, 2042566601, 583504547, 3250051920, 3933861565,
            ],
        ),
    ] {
        for (index, expected) in [0, 1, 2, 7, 8, 15, 16].into_iter().zip(expected) {
            assert_eq!(random_u32_at(seed, index), expected);
            assert_eq!(
                random_noise_at(seed, index),
                (f64::from(expected) / 4294967296.0 - 0.5) as f32
            );
        }
        let mut stream = Mulberry32::new(seed);
        for index in 0..100 {
            assert_eq!(random_u32_at(seed, index), stream.next_u32());
        }
    }
}

#[test]
fn global_random_index_wraps_only_at_the_defined_u32_sequence_period() {
    for (seed, last_draw) in [(0, 0), (1, 63), (u32::MAX, 142530043)] {
        assert_eq!(random_u32_at(seed, u64::from(u32::MAX)), last_draw);
        assert_eq!(random_u32_at(seed, 1_u64 << 32), random_u32_at(seed, 0));
    }
    assert_ne!(random_noise_at(0, 16), random_noise_at(1, 16));
}

#[test]
fn bayer_rgba8_reconstruction_matches_a_hand_calculated_tile_and_preserves_hidden_rgb() {
    let data = [
        128, 128, 128, 0, 128, 128, 128, 7, 128, 128, 128, 128, 128, 128, 128, 255,
    ];
    let output = perturb(
        source(&data, 2, 2),
        policy(FIELDS[0], WorkingSpace::Srgb, 1.0),
    )
    .unwrap();
    // Centered thresholds [-3/8,1/8,3/8,-1/8] times 0.25 times 255, then round.
    assert_eq!(
        output.data(),
        &[104, 104, 104, 0, 136, 136, 136, 7, 152, 152, 152, 128, 120, 120, 120, 255]
    );
    let clipped = [0, 255, 128, 0];
    assert_eq!(
        perturb(
            source(&clipped, 1, 1),
            policy(FIELDS[0], WorkingSpace::Srgb, 1.0)
        )
        .unwrap()
        .data(),
        &[0, 231, 104, 0]
    );
}

#[test]
fn zero_strength_and_zero_adaptive_mask_are_byte_identity_in_every_space() {
    let data = [
        1, 10, 255, 0, 128, 12, 67, 127, 0, 255, 0, 255, 254, 1, 11, 3,
    ];
    for space in SPACES {
        for field in FIELDS {
            assert_eq!(
                perturb(source(&data, 2, 2), policy(field, space, 0.0))
                    .unwrap()
                    .data(),
                &data
            );
            let mut settings = policy(field, space, 1.0);
            settings.placement = Placement::Adaptive {
                radius: 1,
                threshold: f32::MAX,
                softness: 0.0,
            };
            assert_eq!(
                perturb(source(&data, 2, 2), settings).unwrap().data(),
                &data
            );
        }
    }
}

#[test]
fn adaptive_mask_scales_the_field_before_reconstruction() {
    let data = [128, 128, 128, 0];
    let mut settings = policy(FIELDS[0], WorkingSpace::Srgb, 1.0);
    settings.placement = Placement::Adaptive {
        radius: 32768,
        threshold: 0.0,
        softness: 10.0,
    };
    // Uniform contrast zero lies at this smoothstep's midpoint, giving half the Bayer offset.
    assert_eq!(
        perturb(source(&data, 1, 1), settings).unwrap().data(),
        &[116, 116, 116, 0]
    );
}

#[test]
fn reordered_bands_match_full_images_in_all_spaces_and_preserve_padding() {
    let dimensions = ImageDimensions::new(17, 5).unwrap();
    let mut data = Vec::new();
    for y in 0..5 {
        for x in 0..17 {
            data.extend_from_slice(&[
                (x * 19 + y * 23) as u8,
                (x * 53 + y * 7) as u8,
                (x * 11 + y * 31) as u8,
                (x * 17) as u8,
            ]);
        }
    }
    let original = data.clone();
    let mut padded_source = Vec::new();
    for (y, row) in data.chunks_exact(17 * 4).enumerate() {
        padded_source.extend_from_slice(row);
        if y < 4 {
            padded_source.extend_from_slice(&[83; 3]);
        }
    }
    for space in SPACES {
        for field in FIELDS {
            let mut settings = policy(field, space, 0.75);
            settings.placement = Placement::Adaptive {
                radius: 2,
                threshold: 3.0,
                softness: 4.0,
            };
            let source = source(&data, 17, 5);
            let full = perturb(source, settings).unwrap();
            let stride = 17 * 4 + 3;
            let strided_source =
                ImageView::new(&padded_source, dimensions, RowStride::new(stride).unwrap())
                    .unwrap();
            let mut bands = ImageBuf::<Rgba8>::from_vec_strided(
                vec![77; stride * 4 + 17 * 4],
                dimensions,
                RowStride::new(stride).unwrap(),
            )
            .unwrap();
            for (start, end) in [(3, 5), (0, 1), (1, 3)] {
                perturb_rows_into(
                    strided_source,
                    bands.as_view_mut(),
                    settings,
                    RowBand::new(start, end).unwrap(),
                );
            }
            for y in 0..5 {
                assert_eq!(
                    bands.as_view().row(y),
                    full.as_view().row(y),
                    "{space:?} {field:?}"
                );
                if y < 4 {
                    assert_eq!(
                        &bands.data()[y as usize * stride + 17 * 4..(y as usize + 1) * stride],
                        &[77; 3]
                    );
                }
            }
        }
    }
    assert_eq!(data, original);
}

#[test]
fn field_callback_draws_once_per_global_pixel_even_when_effect_is_zero() {
    let data = [0; 3 * 3 * 4];
    let source = source(&data, 3, 3);
    let mut output =
        ImageBuf::<Rgba8>::from_vec_packed(vec![91; data.len()], source.dimensions()).unwrap();
    let calls = std::cell::RefCell::new(Vec::new());
    perturb_by_field_rows_into(
        source,
        output.as_view_mut(),
        WorkingSpace::Srgb,
        0.0,
        Placement::Everywhere,
        RowBand::new(1, 2).unwrap(),
        |x, y, index| {
            calls.borrow_mut().push((x, y, index));
            0.25
        },
    );
    assert_eq!(*calls.borrow(), vec![(0, 1, 3), (1, 1, 4), (2, 1, 5)]);
    assert_eq!(&output.data()[..12], &[91; 12]);
    assert_eq!(&output.data()[12..24], &[0; 12]);
    assert_eq!(&output.data()[24..], &[91; 12]);
}

#[test]
fn seeded_outputs_repeat_and_hidden_pixels_keep_their_global_draw_slots() {
    let data = [128, 128, 128, 0, 128, 128, 128, 255, 128, 128, 128, 255];
    let source = source(&data, 3, 1);
    let settings = policy(Field::Random { seed: 0 }, WorkingSpace::Srgb, 1.0);
    let output = perturb(source, settings).unwrap();
    assert_eq!(output, perturb(source, settings).unwrap());
    assert_ne!(
        output,
        perturb(
            source,
            policy(Field::Random { seed: 1 }, WorkingSpace::Srgb, 1.0)
        )
        .unwrap()
    );
    // Independent seed-zero draws in the integer fixture produce these rounded RGB bytes.
    assert_eq!(
        output.data(),
        &[113, 113, 113, 0, 96, 96, 96, 255, 110, 110, 110, 255]
    );
}

#[test]
fn maximum_legal_strength_reconstructs_every_space_without_losing_alpha() {
    let data = [
        128, 32, 240, 0, 10, 200, 30, 7, 255, 0, 127, 128, 0, 255, 0, 255,
    ];
    for space in SPACES {
        for field in FIELDS {
            let output = perturb(source(&data, 2, 2), policy(field, space, f32::MAX)).unwrap();
            for (pixel, original) in output.data().chunks_exact(4).zip(data.chunks_exact(4)) {
                assert_eq!(pixel[3], original[3]);
                assert!(
                    pixel[..3]
                        .iter()
                        .all(|&channel| channel == 0 || channel == 255),
                    "{space:?} {field:?}"
                );
            }
        }
    }
}

#[test]
fn quantize_callback_sees_rounded_bytes_and_palette_changes_do_not_change_the_field() {
    let data = [128, 128, 128, 255];
    let source = source(&data, 1, 1);
    let settings = policy(FIELDS[0], WorkingSpace::Srgb, 1.0);
    let intermediate = perturb(source, settings).unwrap();
    for entries in [
        vec![
            PaletteEntry::Color { rgb: [104; 3] },
            PaletteEntry::Color { rgb: [105; 3] },
        ],
        vec![
            PaletteEntry::Color { rgb: [0; 3] },
            PaletteEntry::Color { rgb: [255; 3] },
        ],
    ] {
        let prepared = PreparedPalette::new(&entries, AlphaPolicy::Premultiplied);
        let result = quantize_after_perturb(source, settings, |bytes| {
            assert_eq!(bytes.data(), intermediate.data());
            assert_eq!(bytes.data(), &[104, 104, 104, 255]);
            let rgba: [u8; 4] = bytes.pixel(0, 0).unwrap().try_into().unwrap();
            let PalettePixel::Color(rgb) = prepared.prepare_pixel(rgba) else {
                panic!("opaque input")
            };
            prepared
                .visible
                .iter()
                .min_by_key(|entry| {
                    rgb.into_iter()
                        .zip(entry.rgb)
                        .map(|(a, b)| (i32::from(a) - i32::from(b)).pow(2))
                        .sum::<i32>()
                })
                .unwrap()
                .index
        })
        .unwrap();
        assert_eq!(result, 0);
    }
}
