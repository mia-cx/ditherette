use ditherette_wasm::{
    image::contracts::{PaletteEntry, Rgba8Image, WarningCode},
    spec::{
        contract::{error::ErrorCode, lifecycle::Stage, request::*},
        pipeline::{self, processor::Processor, ProcessedImage},
        quantize, resize,
    },
};

const DATA: [u8; 24] = [
    128, 128, 128, 255, 80, 30, 240, 128, 7, 9, 11, 0, 240, 30, 80, 255, 50, 200, 70, 1, 255, 255,
    255, 254,
];
const PALETTE: [PaletteEntry; 4] = [
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Transparent {},
    PaletteEntry::Color {
        rgb: [128, 128, 128],
    },
    PaletteEntry::Color {
        rgb: [255, 255, 255],
    },
];
const SPACES: [WorkingSpace; 7] = [
    WorkingSpace::Srgb,
    WorkingSpace::LinearRgb,
    WorkingSpace::Oklab,
    WorkingSpace::Oklch,
    WorkingSpace::Cielab,
    WorkingSpace::Cielch,
    WorkingSpace::Ycbcr,
];
const MATCHES: [MatchPolicy; 15] = [
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
];

fn source() -> Source<'static> {
    Source {
        width: 3,
        height: 2,
        data: &DATA,
    }
}

fn from_image(image: &Rgba8Image) -> Source<'_> {
    Source {
        width: image.dimensions().width(),
        height: image.dimensions().height(),
        data: image.data(),
    }
}

fn matching() -> QuantizeRequest<'static> {
    QuantizeRequest {
        version: 1,
        source: source(),
        palette: &PALETTE,
        alpha: AlphaPolicy::Preserve { threshold: 127.5 },
        matching: MatchPolicy::SrgbEuclidean,
    }
}

fn field(field: Field, space: WorkingSpace) -> PerturbPolicy {
    PerturbPolicy {
        field,
        space,
        strength: 1.0,
        placement: Placement::Adaptive {
            radius: 1,
            threshold: 8.0,
            softness: 12.0,
        },
    }
}

fn resize_request() -> ResizeRequest<'static> {
    ResizeRequest {
        version: 1,
        source: source(),
        output: Output {
            width: 2,
            height: 3,
            resize: ResizePolicy::Area {},
        },
    }
}

#[test]
fn separable_composition_preserves_rgba8_boundary_for_every_field_space_and_metric() {
    let fields = [
        Field::Bayer {
            size: BayerSize::Two,
        },
        Field::Bayer {
            size: BayerSize::Four,
        },
        Field::Bayer {
            size: BayerSize::Eight,
        },
        Field::Bayer {
            size: BayerSize::Sixteen,
        },
        Field::Random { seed: 0xd17ee77e },
        Field::BlueNoise {},
    ];
    for field_kind in fields {
        for space in SPACES {
            let perturb = field(field_kind, space);
            let intermediate = pipeline::perturb(PerturbRequest {
                version: 1,
                source: source(),
                perturb,
            })
            .unwrap();
            for matching_policy in MATCHES {
                let quantize = QuantizeRequest {
                    matching: matching_policy,
                    ..matching()
                };
                let separate = quantize::quantize(QuantizeRequest {
                    source: from_image(&intermediate),
                    ..quantize
                })
                .unwrap();
                let fused = pipeline::dither_and_quantize(DitherQuantizeRequest {
                    quantize,
                    dither: DitherPolicy::Separable { perturb },
                })
                .unwrap();
                assert_eq!(
                    fused, separate,
                    "{field_kind:?} {space:?} {matching_policy:?}"
                );
            }
        }
    }
}

#[test]
fn process_equals_resize_then_every_dither_family_with_identical_metadata() {
    let filters = [
        ResizePolicy::Nearest {
            anchor: Anchor::BottomRight,
        },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::TopLeft,
            support: Support::ScaleAware,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Bottom,
            support: Support::ScaleAware,
        },
        ResizePolicy::Trilinear {
            anchor: Anchor::Right,
        },
    ];
    let mut dithers = vec![
        DitherPolicy::None {},
        DitherPolicy::Separable {
            perturb: field(Field::BlueNoise {}, WorkingSpace::Oklch),
        },
    ];
    for kernel in [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
    ] {
        for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
            for serpentine in [false, true] {
                dithers.push(DitherPolicy::Diffusion {
                    kernel,
                    feedback,
                    serpentine,
                    strength: 0.8,
                    placement: Placement::Everywhere {},
                });
            }
        }
    }
    for size in [
        BayerSize::Two,
        BayerSize::Four,
        BayerSize::Eight,
        BayerSize::Sixteen,
    ] {
        dithers.push(DitherPolicy::Yliluoma {
            size,
            placement: field(Field::BlueNoise {}, WorkingSpace::Srgb).placement,
        });
    }
    for filter in filters {
        let output = Output {
            resize: filter,
            ..resize_request().output
        };
        let resized = resize::resize(ResizeRequest {
            output,
            ..resize_request()
        })
        .unwrap();
        for &dither in &dithers {
            let request = ProcessRequest {
                source: source(),
                palette: &PALETTE,
                recipe: RecipeV1 {
                    version: 1,
                    output,
                    alpha: matching().alpha,
                    matching: matching().matching,
                    dither,
                },
            };
            let composed = pipeline::dither_and_quantize(DitherQuantizeRequest {
                quantize: QuantizeRequest {
                    source: from_image(&resized),
                    ..matching()
                },
                dither,
            })
            .unwrap();
            assert_eq!(
                pipeline::process(request).unwrap(),
                composed,
                "{filter:?} {dither:?}"
            );
        }
    }
}

#[test]
fn joined_blue_noise_dispatch_has_independently_known_first_row_bytes() {
    let bytes = [
        128, 128, 128, 0, 128, 128, 128, 1, 128, 128, 128, 127, 128, 128, 128, 255,
    ];
    let image = pipeline::perturb(PerturbRequest {
        version: 1,
        source: Source {
            width: 4,
            height: 1,
            data: &bytes,
        },
        perturb: PerturbPolicy {
            placement: Placement::Everywhere {},
            ..field(Field::BlueNoise {}, WorkingSpace::Srgb)
        },
    })
    .unwrap();
    assert_eq!(
        image.data(),
        [115, 115, 115, 0, 144, 144, 144, 1, 96, 96, 96, 127, 151, 151, 151, 255]
    );
}

#[test]
fn truncation_and_fallback_warnings_survive_process_composition() {
    let palette = [PaletteEntry::Color { rgb: [0, 0, 0] }; 257];
    let request = ProcessRequest {
        source: source(),
        palette: &palette,
        recipe: RecipeV1 {
            version: 1,
            output: resize_request().output,
            alpha: matching().alpha,
            matching: matching().matching,
            dither: DitherPolicy::None {},
        },
    };
    let result = pipeline::process(request).unwrap();
    assert_eq!(
        result
            .warnings
            .iter()
            .map(|warning| warning.code)
            .collect::<Vec<_>>(),
        [
            WarningCode::PaletteTruncated,
            WarningCode::TransparentFallback
        ]
    );
    assert_eq!(result.palette.rgba.len(), 1024);
    let resized = resize::resize(resize_request()).unwrap();
    assert_eq!(
        result,
        quantize::quantize(QuantizeRequest {
            source: from_image(&resized),
            palette: &palette,
            ..matching()
        })
        .unwrap()
    );
}

fn requests() -> [Request<'static>; 5] {
    [
        Request::Resize(resize_request()),
        Request::Perturb(PerturbRequest {
            version: 1,
            source: source(),
            perturb: field(Field::Random { seed: 42 }, WorkingSpace::Cielab),
        }),
        Request::Quantize(matching()),
        Request::DitherAndQuantize(DitherQuantizeRequest {
            quantize: matching(),
            dither: DitherPolicy::None {},
        }),
        Request::Process(ProcessRequest {
            source: source(),
            palette: &PALETTE,
            recipe: RecipeV1 {
                version: 1,
                output: resize_request().output,
                alpha: matching().alpha,
                matching: matching().matching,
                dither: DitherPolicy::Separable {
                    perturb: field(Field::BlueNoise {}, WorkingSpace::LinearRgb),
                },
            },
        }),
    ]
}

#[test]
fn every_method_rejects_callback_reentry_and_disposal_without_disrupting_the_call() {
    let processor = Processor::default();
    for request in requests() {
        let mut stages = Vec::new();
        let mut callback = |progress: ditherette_wasm::spec::contract::lifecycle::Progress| {
            stages.push(progress.stage);
            assert_eq!(
                processor.execute(request, None, || 0).unwrap_err().code,
                ErrorCode::ReentrantCall
            );
            assert_eq!(
                processor.dispose().unwrap_err().code,
                ErrorCode::ReentrantCall
            );
            Ok(())
        };
        let output = processor
            .execute(request, Some(&mut callback), || 0)
            .unwrap();
        assert_eq!(output, pipeline::execute(request).unwrap());
        assert_eq!(stages.first(), Some(&Stage::Prepare));
        assert_eq!(stages.last(), Some(&Stage::Complete));
        assert!(stages.windows(2).all(|pair| pair[0] != pair[1]));
    }
}

#[test]
fn callback_failure_at_every_emitted_stage_returns_no_result_and_leaves_instance_usable() {
    let processor = Processor::default();
    for request in requests() {
        let mut stages = Vec::new();
        processor
            .execute(
                request,
                Some(&mut |progress| {
                    stages.push(progress.stage);
                    Ok(())
                }),
                || 0,
            )
            .unwrap();
        for failed_stage in stages {
            let error = processor
                .execute(
                    request,
                    Some(&mut |progress| {
                        if progress.stage == failed_stage {
                            Err(())
                        } else {
                            Ok(())
                        }
                    }),
                    || 0,
                )
                .unwrap_err();
            assert_eq!(
                (error.code, error.path.as_str()),
                (ErrorCode::Callback, "onProgress")
            );
            assert_eq!(
                processor.execute(request, None, || 0).unwrap(),
                pipeline::execute(request).unwrap()
            );
        }
    }
}

#[test]
fn invalid_calls_emit_no_progress_and_do_not_poison_the_instance() {
    let processor = Processor::default();
    let request = Request::Quantize(QuantizeRequest {
        palette: &[],
        ..matching()
    });
    let mut callbacks = 0;
    let error = processor
        .execute(
            request,
            Some(&mut |_| {
                callbacks += 1;
                Ok(())
            }),
            || 0,
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidPalette);
    assert_eq!(callbacks, 0);
    assert!(processor
        .execute(Request::Quantize(matching()), None, || 0)
        .is_ok());
}

#[test]
fn measured_stage_counts_obey_the_throttle_but_completion_is_immediate() {
    for elapsed in [49, 50] {
        let processor = Processor::default();
        let mut events = Vec::new();
        let mut clock = [0, 0, elapsed, elapsed].into_iter();
        processor
            .execute(
                Request::Resize(resize_request()),
                Some(&mut |progress| {
                    events.push(progress);
                    Ok(())
                }),
                || clock.next().expect("four reference progress opportunities"),
            )
            .unwrap();
        let resize_events = events
            .iter()
            .filter(|event| event.stage == Stage::Resize)
            .collect::<Vec<_>>();
        assert_eq!(resize_events.len(), if elapsed == 49 { 1 } else { 2 });
        assert_eq!(resize_events[0].completed, Some(0));
        assert!(resize_events.iter().all(|event| event.total == Some(6)));
        if elapsed == 50 {
            assert_eq!(resize_events[1].completed, Some(6));
        }
        let complete = events.last().unwrap();
        assert_eq!(
            (complete.stage, complete.completed, complete.total),
            (Stage::Complete, Some(6), Some(6))
        );
    }
}

#[test]
fn kernel_arithmetic_failure_never_completes_or_poisons_the_processor() {
    let processor = Processor::default();
    let bytes = [100, 100, 100, 255, 100, 100, 100, 255];
    let palette = [
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
    ];
    for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
        let request = Request::DitherAndQuantize(DitherQuantizeRequest {
            quantize: QuantizeRequest {
                source: Source {
                    width: 2,
                    height: 1,
                    data: &bytes,
                },
                palette: &palette,
                ..matching()
            },
            dither: DitherPolicy::Diffusion {
                kernel: Diffusion::FloydSteinberg,
                strength: f32::MAX,
                placement: Placement::Everywhere {},
                serpentine: false,
                feedback,
            },
        });
        let mut stages = Vec::new();
        let error = processor
            .execute(
                request,
                Some(&mut |progress| {
                    stages.push(progress.stage);
                    Ok(())
                }),
                || 0,
            )
            .unwrap_err();
        assert_eq!(
            (error.code, error.path.as_str()),
            (ErrorCode::Runtime, "dither.arithmetic")
        );
        assert!(!stages.contains(&Stage::Complete));
        assert!(processor
            .execute(Request::Quantize(matching()), None, || 0)
            .is_ok());
    }
}

#[test]
fn returned_bytes_and_metadata_survive_other_calls_input_mutation_and_disposal() {
    let mut source_bytes = DATA;
    let processor = Processor::default();
    let request = Request::Quantize(QuantizeRequest {
        source: Source {
            data: &source_bytes,
            ..source()
        },
        ..matching()
    });
    let result = processor.execute(request, None, || 0).unwrap();
    let original = result.clone();
    assert_eq!(source_bytes, DATA);
    source_bytes.fill(255);
    for request in requests() {
        processor.execute(request, None, || 0).unwrap();
    }
    processor.dispose().unwrap();
    processor.dispose().unwrap();
    assert_eq!(result, original);
    assert!(matches!(result, ProcessedImage::Indexed(_)));
    assert_eq!(
        processor
            .execute(Request::Quantize(matching()), None, || 0)
            .unwrap_err()
            .code,
        ErrorCode::Disposed
    );
    assert!(Processor::default()
        .execute(Request::Quantize(matching()), None, || 0)
        .is_ok());
}
