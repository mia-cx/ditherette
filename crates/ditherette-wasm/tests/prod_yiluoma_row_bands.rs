use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions, ImageView, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{AlphaPolicy, MatchPolicy, Placement},
        },
        dither::{
            ordered::BayerSize,
            yiluoma::{dither_yiluoma_into, row_bands::YliluomaBands},
        },
        quantize::PreparedQuantizer,
        tiling::WorkerBudget,
    },
};

fn pixels(dimensions: ImageDimensions) -> Vec<u8> {
    (0..dimensions.pixel_count().unwrap())
        .flat_map(|i| {
            [
                (i * 37) as u8,
                (i * 113 + 7) as u8,
                (i * 11 + 83) as u8,
                [0, 127, 128, 255][i % 4],
            ]
        })
        .collect()
}

#[test]
fn all_metrics_and_matrices_match_scalar_on_tiny_adaptive_bands() {
    let dimensions = ImageDimensions::new(2, 3).unwrap();
    let bytes = pixels(dimensions);
    let source = ImageView::<Rgba8>::packed(&bytes, dimensions).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [255, 3, 0] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [255, 0, 3] },
    ];
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ] {
        let prepared = PreparedQuantizer::try_new(
            &palette,
            AlphaPolicy::Matte { rgb: [29, 71, 211] },
            matching,
            u64::MAX,
        )
        .unwrap();
        for size in [
            BayerSize::Two,
            BayerSize::Four,
            BayerSize::Eight,
            BayerSize::Sixteen,
        ] {
            let placement = Placement::Adaptive {
                radius: 1,
                threshold: 5.0,
                softness: 10.0,
            };
            compare(source, &prepared, size, placement, &[1, 2, 8], &[1, 2, 9]);
        }
    }
}

#[test]
fn uneven_bands_keep_global_bayer_alpha_ties_and_original_source_neighbors() {
    let dimensions = ImageDimensions::new(5, 7).unwrap();
    let bytes = pixels(dimensions);
    let source = ImageView::<Rgba8>::packed(&bytes, dimensions).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [128; 3] },
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [64; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    for palette in [&palette[..1], &palette[..4], &palette[..], &palette[2..3]] {
        for alpha in [
            AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [29, 71, 211] },
        ] {
            let prepared =
                PreparedQuantizer::try_new(palette, alpha, MatchPolicy::SrgbEuclidean, u64::MAX)
                    .unwrap();
            for placement in [
                Placement::Everywhere {},
                Placement::Adaptive {
                    radius: 2,
                    threshold: 5.0,
                    softness: 10.0,
                },
                Placement::Adaptive {
                    radius: 32768,
                    threshold: 100.0,
                    softness: 0.0,
                },
            ] {
                compare(
                    source,
                    &prepared,
                    BayerSize::Four,
                    placement,
                    &[1, 2, 4, 8],
                    &[1, 3, 99],
                );
            }
        }
    }
}

fn compare(
    source: ImageView<'_, Rgba8>,
    prepared: &PreparedQuantizer,
    size: BayerSize,
    placement: Placement,
    workers: &[u32],
    heights: &[u32],
) {
    let dimensions = source.dimensions();
    let mut expected = vec![0; dimensions.pixel_count().unwrap()];
    dither_yiluoma_into(source, prepared, &mut expected, size, placement);
    for &workers in workers {
        for &height in heights {
            let mut bands =
                YliluomaBands::try_new(dimensions, height, WorkerBudget::new(8), workers, u64::MAX)
                    .unwrap();
            let mut actual = vec![255; expected.len()];
            let caller = std::thread::current().id();
            let mut completed = 0;
            bands
                .execute(
                    source,
                    prepared,
                    &mut actual,
                    size,
                    placement,
                    &mut |rows| {
                        assert_eq!(std::thread::current().id(), caller);
                        assert!(rows > completed && rows <= dimensions.height());
                        completed = rows;
                        Ok(())
                    },
                )
                .unwrap();
            assert_eq!(completed, dimensions.height());
            assert_eq!(
                actual, expected,
                "{workers}/{height}/{size:?}/{placement:?}"
            );
        }
    }
}

#[test]
fn exact_budget_covers_metadata_and_concurrent_temporary_converters() {
    let dimensions = ImageDimensions::new(3, 7).unwrap();
    for workers in [1, 2, 8] {
        for height in [1, 3, 99] {
            let pool = WorkerBudget::new(4);
            let required =
                YliluomaBands::required_bytes(dimensions, height, pool, workers).unwrap();
            let bands =
                YliluomaBands::try_new(dimensions, height, pool, workers, required).unwrap();
            assert_eq!(bands.capacity_bytes(), required);
            let error = YliluomaBands::try_new(dimensions, height, pool, workers, required - 1)
                .err()
                .unwrap();
            assert_eq!(error.code, ErrorCode::MemoryLimit);
        }
    }
    assert_eq!(
        YliluomaBands::required_bytes(dimensions, 0, WorkerBudget::new(1), 1)
            .unwrap_err()
            .code,
        ErrorCode::InvalidImage,
    );
}

#[test]
fn callback_failure_joins_current_batch_and_adapter_can_be_reused() {
    let dimensions = ImageDimensions::new(2, 7).unwrap();
    let bytes = pixels(dimensions);
    let source = ImageView::<Rgba8>::packed(&bytes, dimensions).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let prepared = PreparedQuantizer::try_new(
        &palette,
        AlphaPolicy::Premultiplied {},
        MatchPolicy::SrgbEuclidean,
        u64::MAX,
    )
    .unwrap();
    let mut bands =
        YliluomaBands::try_new(dimensions, 1, WorkerBudget::new(2), 2, u64::MAX).unwrap();
    let mut actual = vec![255; dimensions.pixel_count().unwrap()];
    let error = bands
        .execute(
            source,
            &prepared,
            &mut actual,
            BayerSize::Two,
            Placement::Everywhere {},
            &mut |_| Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress)),
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Callback);
    assert!(actual.contains(&255), "later batches remain unexecuted");
    bands
        .execute(
            source,
            &prepared,
            &mut actual,
            BayerSize::Two,
            Placement::Everywhere {},
            &mut |_| Ok(()),
        )
        .unwrap();
    let mut expected = vec![0; actual.len()];
    dither_yiluoma_into(
        source,
        &prepared,
        &mut expected,
        BayerSize::Two,
        Placement::Everywhere {},
    );
    assert_eq!(actual, expected);
}
