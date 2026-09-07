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
fn all_ordinary_modes_match_indices_palette_alpha_and_warning_bytes() {
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
    for (matching, ..) in MODES {
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
fn unsupported_matching_is_not_silently_euclidean() {
    let palette = [PaletteEntry::Transparent {}];
    let request = request(
        &[0, 0, 0, 255],
        &palette,
        MatchPolicy::OklchHueArc,
        AlphaPolicy::Preserve { threshold: 0.0 },
    );
    let prod::quantize::QuantizeError::Preparation(failure) =
        prod::quantize::quantize(request, u64::MAX).unwrap_err()
    else {
        panic!("unsupported matching must be a preparation failure");
    };
    assert_eq!(
        failure.code,
        prod::contract::error::ErrorCode::UnsupportedOperation
    );
    assert_eq!(failure.path, "matching");
}
