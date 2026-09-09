use ditherette_bench::verification::{
    content_digest, input_digest, settings_digest, verify_three_way, VerificationBounds,
    VerificationStatus,
};
use ditherette_bench_api::{verification::*, ConformanceBenchSubject, PixelFormat};
use ditherette_wasm::{
    bench_subjects::{
        reference::{ReferenceFn, ReferenceRequest},
        BenchSubject,
    },
    spec::contract::request::*,
};

fn subject(id: &str) -> ConformanceBenchSubject<ReferenceFn> {
    ditherette_wasm::bench_subjects()
        .into_iter()
        .find_map(|subject| match subject {
            BenchSubject::Conformance(subject) if subject.descriptor.id.as_str() == id => {
                Some(subject)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing callable reference {id}"))
}

fn case(request: ReferenceRequest<'_>) -> VerificationCase<ReferenceRequest<'_>> {
    let source = request.source();
    VerificationCase {
        identity: CaseIdentity {
            semantics: request.semantics(),
            input: input_digest(
                Dimensions {
                    width: source.width,
                    height: source.height,
                },
                source.data,
            ),
            settings: settings_digest(&request).unwrap(),
            output: request.dimensions().unwrap(),
        },
        request,
    }
}

#[test]
fn seven_discoverable_color_pairs_keep_packed_coordinates_alpha_and_f32_inverse() {
    let rgba = [
        0, 0, 0, 0, 255, 255, 255, 17, 255, 0, 0, 255, 10, 11, 128, 127,
    ];
    let source = Source {
        width: 4,
        height: 1,
        data: &rgba,
    };
    for (name, space) in [
        ("srgb", WorkingSpace::Srgb),
        ("linear-rgb", WorkingSpace::LinearRgb),
        ("oklab", WorkingSpace::Oklab),
        ("oklch", WorkingSpace::Oklch),
        ("cielab", WorkingSpace::Cielab),
        ("cielch", WorkingSpace::Cielch),
        ("ycbcr", WorkingSpace::Ycbcr),
    ] {
        let subject = subject(&format!("spec:color:{name}:f32-roundtrip-v1"));
        assert_eq!(subject.operation, Operation::Color);
        assert_eq!(
            subject.descriptor.capabilities.pixel_formats,
            [PixelFormat::Color32]
        );
        let output = (subject.run)(&ReferenceRequest::Color { source, space }).unwrap();
        let Pixels::Color {
            coordinates,
            alpha,
            rendered_rgba,
            ..
        } = output.pixels
        else {
            panic!("color result required")
        };
        assert_eq!(coordinates.len(), 12);
        assert_eq!(alpha, [0, 17, 255, 127]);
        assert_eq!(rendered_rgba.unwrap(), rgba);
        if space == WorkingSpace::Srgb || space == WorkingSpace::LinearRgb {
            assert_eq!(&coordinates[..6], &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        }
        let wrong_space = if space == WorkingSpace::Srgb {
            WorkingSpace::LinearRgb
        } else {
            WorkingSpace::Srgb
        };
        assert!((subject.run)(&ReferenceRequest::Color {
            source,
            space: wrong_space
        })
        .is_err());
    }
}

#[test]
fn concrete_reference_records_do_not_fill_missing_production_roles() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
    ];
    let request = ReferenceRequest::Processing(Request::Quantize(QuantizeRequest {
        version: 1,
        source: Source {
            width: 2,
            height: 1,
            data: &[0, 0, 0, 0, 255, 255, 255, 255],
        },
        palette: &palette,
        alpha: AlphaPolicy::Preserve { threshold: 1.0 },
        matching: MatchPolicy::SrgbEuclidean,
    }));
    let case = case(request);
    let registered = subject("spec:quantize:request:v1");
    let reference = VerificationSubject {
        identity: ImplementationIdentity {
            subject: registered.descriptor.id.to_string(),
            artifact: ArtifactIdentity {
                revision: "a".repeat(40),
                content: content_digest(b"explicit fixture artifact"),
            },
        },
        semantics: case.identity.semantics.clone(),
        run: registered.run,
    }
    .evaluate(&case)
    .unwrap();
    let Pixels::Indexed8 {
        indices,
        palette_rgba,
        transparent_index,
    } = &reference.output.pixels
    else {
        panic!("indexed output required")
    };
    assert_eq!(indices, &[0, 1]);
    assert_eq!(palette_rgba, &[0, 0, 0, 0, 255, 255, 255, 255]);
    assert_eq!(*transparent_index, Some(0));
    let report = verify_three_way(
        &case.identity,
        &ThreeWayOutputs {
            reference_state: ReferenceState::PreFreeze,
            reference: Some(reference),
            accepted: None,
            candidate: None,
        },
        VerificationBounds::exact(),
    );
    assert_eq!(report.status, VerificationStatus::Incomplete);
    assert!(!report.release_conformant());
}

#[test]
fn settings_identity_binds_independent_perturb_and_matching_spaces() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    let palette = [PaletteEntry::Color { rgb: [0, 0, 0] }];
    let source = Source {
        width: 1,
        height: 1,
        data: &[128, 128, 128, 255],
    };
    let mut request = DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source,
            palette: &palette,
            alpha: AlphaPolicy::Premultiplied {},
            matching: MatchPolicy::CielabEuclidean,
        },
        dither: DitherPolicy::Separable {
            perturb: PerturbPolicy {
                field: Field::Random { seed: 7 },
                space: WorkingSpace::Srgb,
                strength: 1.0,
                placement: Placement::Everywhere {},
            },
        },
    };
    let original = case(ReferenceRequest::Processing(Request::DitherAndQuantize(
        request,
    )));
    if let DitherPolicy::Separable { perturb } = &mut request.dither {
        perturb.space = WorkingSpace::Oklab;
    }
    let changed = case(ReferenceRequest::Processing(Request::DitherAndQuantize(
        request,
    )));
    assert_eq!(original.identity.semantics.space, Some(ColorSpace::Cielab));
    assert_eq!(original.identity.semantics, changed.identity.semantics);
    assert_ne!(original.identity.settings, changed.identity.settings);
    request.quantize.matching = MatchPolicy::SrgbEuclidean;
    let matching = case(ReferenceRequest::Processing(Request::DitherAndQuantize(
        request,
    )));
    assert_ne!(changed.identity.settings, matching.identity.settings);
    assert_ne!(
        changed.identity.semantics.space,
        matching.identity.semantics.space
    );
}

fn run(method: &str, request: Request<'_>) -> VerificationOutput {
    let subject = subject(&format!("spec:{method}:request:v1"));
    (subject.run)(&ReferenceRequest::Processing(request)).unwrap()
}

#[test]
fn all_five_registry_methods_preserve_byte_composition_and_warning_metadata() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    let registered = ditherette_wasm::bench_subjects();
    let ids: std::collections::BTreeSet<_> = registered
        .iter()
        .map(|entry| entry.descriptor().id.to_string())
        .collect();
    assert_eq!(ids.len(), registered.len());
    assert_eq!(
        registered
            .iter()
            .filter(|entry| matches!(entry, BenchSubject::Conformance(_))
                && entry.descriptor().id.module() == "spec")
            .count(),
        24
    );

    let rgba = [
        128, 128, 128, 0, 128, 128, 128, 7, 128, 128, 128, 128, 128, 128, 128, 255,
    ];
    let source = Source {
        width: 2,
        height: 2,
        data: &rgba,
    };
    let mut palette = vec![
        PaletteEntry::Color {
            rgb: [255, 255, 255]
        };
        257
    ];
    palette[0] = PaletteEntry::Transparent {};
    palette[1] = PaletteEntry::Color { rgb: [0, 0, 0] };
    let output = Output {
        width: 2,
        height: 2,
        resize: ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
    };
    let perturb = PerturbPolicy {
        field: Field::Bayer {
            size: BayerSize::Two,
        },
        space: WorkingSpace::Srgb,
        strength: 1.0,
        placement: Placement::Everywhere {},
    };
    let alpha = AlphaPolicy::Preserve { threshold: 1.0 };
    let matching = MatchPolicy::SrgbEuclidean;
    let quantize = QuantizeRequest {
        version: 1,
        source,
        palette: &palette,
        alpha,
        matching,
    };
    let resized = run(
        "resize",
        Request::Resize(ResizeRequest {
            version: 1,
            source,
            output,
        }),
    );
    assert_eq!(
        resized.pixels,
        Pixels::Rgba8 {
            data: rgba.to_vec()
        }
    );
    let perturbed = run(
        "perturb",
        Request::Perturb(PerturbRequest {
            version: 1,
            source,
            perturb,
        }),
    );
    let Pixels::Rgba8 { data } = &perturbed.pixels else {
        panic!("RGBA8 boundary required")
    };
    assert_eq!(
        data,
        &[104, 104, 104, 0, 136, 136, 136, 7, 152, 152, 152, 128, 120, 120, 120, 255]
    );
    let composed = run(
        "quantize",
        Request::Quantize(QuantizeRequest {
            source: Source { data, ..source },
            ..quantize
        }),
    );
    let dither = DitherPolicy::Separable { perturb };
    let fused = run(
        "dither-and-quantize",
        Request::DitherAndQuantize(DitherQuantizeRequest { quantize, dither }),
    );
    let processed = run(
        "process",
        Request::Process(ProcessRequest {
            source,
            palette: &palette,
            recipe: RecipeV1 {
                version: 1,
                output,
                alpha,
                matching,
                dither,
            },
        }),
    );
    assert_eq!(composed, fused);
    assert_eq!(processed, fused);
    let Pixels::Indexed8 {
        indices,
        palette_rgba,
        transparent_index,
    } = &processed.pixels
    else {
        panic!("indexed result required")
    };
    assert_eq!(indices, &[0, 2, 2, 1]);
    assert_eq!(palette_rgba.len(), 256 * 4);
    assert_eq!(*transparent_index, Some(0));
    assert_eq!(processed.warnings.len(), 1);
    assert_eq!(processed.warnings[0].code, WarningCode::PaletteTruncated);
    assert!(!processed.warnings[0].message.is_empty());
    let wrong = ReferenceRequest::Processing(Request::Quantize(quantize));
    assert!((subject("spec:process:request:v1").run)(&wrong).is_err());
    let invalid = ReferenceRequest::Processing(Request::Quantize(QuantizeRequest {
        source: Source {
            data: &[],
            ..source
        },
        ..quantize
    }));
    assert!((subject("spec:quantize:request:v1").run)(&invalid).is_err());
    assert_eq!(source.data, rgba);
}

#[test]
fn typed_registration_accepts_every_reference_mode_without_timing() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    let rgba = [
        20, 40, 60, 255, 80, 100, 120, 127, 140, 160, 180, 0, 200, 220, 240, 255,
    ];
    let source = Source {
        width: 2,
        height: 2,
        data: &rgba,
    };
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [0, 0, 0] },
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
    ];
    let mut resize_modes = vec![ResizePolicy::Area {}];
    for anchor in [
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
    ] {
        resize_modes.extend([
            ResizePolicy::Nearest { anchor },
            ResizePolicy::Bilinear { anchor },
            ResizePolicy::Trilinear { anchor },
        ]);
        for support in [Support::Fixed, Support::ScaleAware] {
            resize_modes.extend([
                ResizePolicy::Bicubic { anchor, support },
                ResizePolicy::Lanczos2 { anchor, support },
                ResizePolicy::Lanczos3 { anchor, support },
            ]);
        }
    }
    for resize in resize_modes {
        let result = run(
            "resize",
            Request::Resize(ResizeRequest {
                version: 1,
                source,
                output: Output {
                    width: 3,
                    height: 2,
                    resize,
                },
            }),
        );
        assert_eq!(
            result.dimensions,
            Dimensions {
                width: 3,
                height: 2
            }
        );
    }
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
        for alpha in [
            AlphaPolicy::Preserve { threshold: 127.5 },
            AlphaPolicy::Premultiplied {},
            AlphaPolicy::Matte { rgb: [31, 61, 91] },
        ] {
            let result = run(
                "quantize",
                Request::Quantize(QuantizeRequest {
                    version: 1,
                    source,
                    palette: &palette,
                    alpha,
                    matching,
                }),
            );
            assert!(matches!(result.pixels, Pixels::Indexed8 { .. }));
        }
    }
    let mut dithers = vec![DitherPolicy::None {}];
    for placement in [
        Placement::Everywhere {},
        Placement::Adaptive {
            radius: 1,
            threshold: 12.0,
            softness: 5.0,
        },
    ] {
        for field in [
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
            Field::Random { seed: 123 },
            Field::BlueNoise {},
        ] {
            for space in [
                WorkingSpace::Srgb,
                WorkingSpace::LinearRgb,
                WorkingSpace::Oklab,
                WorkingSpace::Oklch,
                WorkingSpace::Cielab,
                WorkingSpace::Cielch,
                WorkingSpace::Ycbcr,
            ] {
                let perturb = PerturbPolicy {
                    field,
                    space,
                    strength: 0.75,
                    placement,
                };
                let result = run(
                    "perturb",
                    Request::Perturb(PerturbRequest {
                        version: 1,
                        source,
                        perturb,
                    }),
                );
                let Pixels::Rgba8 { data } = result.pixels else {
                    panic!("RGBA8 expected")
                };
                assert_eq!(
                    data.chunks_exact(4).map(|p| p[3]).collect::<Vec<_>>(),
                    [255, 127, 0, 255]
                );
                dithers.push(DitherPolicy::Separable { perturb });
            }
        }
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
                        strength: 0.75,
                        placement,
                        serpentine,
                        feedback,
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
            dithers.push(DitherPolicy::Yliluoma { size, placement });
        }
    }
    for dither in dithers {
        let alpha = AlphaPolicy::Preserve { threshold: 128.0 };
        let matching = MatchPolicy::OklabEuclidean;
        let quantize = QuantizeRequest {
            version: 1,
            source,
            palette: &palette,
            alpha,
            matching,
        };
        let fused = run(
            "dither-and-quantize",
            Request::DitherAndQuantize(DitherQuantizeRequest { quantize, dither }),
        );
        let processed = run(
            "process",
            Request::Process(ProcessRequest {
                source,
                palette: &palette,
                recipe: RecipeV1 {
                    version: 1,
                    output: Output {
                        width: 2,
                        height: 2,
                        resize: ResizePolicy::Nearest {
                            anchor: Anchor::Center,
                        },
                    },
                    alpha,
                    matching,
                    dither,
                },
            }),
        );
        assert_eq!(fused, processed);
    }
    assert_eq!(source.data, rgba);
}
