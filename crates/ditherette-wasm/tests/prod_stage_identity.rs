use ditherette_wasm::{
    image::contracts::PaletteEntry,
    prod::{
        contract::{
            cache::{source_identity, Identity, StageOptions},
            request::*,
        },
        pipeline::identity,
    },
    spec::contract::{cache as frozen, request as reference},
};

fn frozen_stage(options: StageOptions) -> frozen::StageOptions {
    let convert = |value| serde_json::from_value(value).unwrap();
    match options {
        StageOptions::Resize { output } => frozen::StageOptions::Resize {
            output: convert(serde_json::to_value(output).unwrap()),
        },
        StageOptions::Perturb { perturb } => frozen::StageOptions::Perturb {
            perturb: serde_json::from_value(serde_json::to_value(perturb).unwrap()).unwrap(),
        },
        StageOptions::Color { space } => frozen::StageOptions::Color {
            space: serde_json::from_value(serde_json::to_value(space).unwrap()).unwrap(),
        },
        StageOptions::Alpha { palette, alpha } => frozen::StageOptions::Alpha {
            palette: frozen::Identity(palette.0),
            alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap()).unwrap(),
        },
        StageOptions::Indexed {
            palette,
            alpha,
            matching,
            dither,
        } => frozen::StageOptions::Indexed {
            palette: frozen::Identity(palette.0),
            alpha: serde_json::from_value(serde_json::to_value(alpha).unwrap()).unwrap(),
            matching: serde_json::from_value(serde_json::to_value(matching).unwrap()).unwrap(),
            dither: serde_json::from_value(serde_json::to_value(dither).unwrap()).unwrap(),
        },
        _ => unreachable!("preparation stages have their existing dedicated comparison"),
    }
}

#[test]
fn materialized_stage_keys_match_the_frozen_canonical_encoding() {
    let entries = [
        PaletteEntry::Color { rgb: [3, 4, 5] },
        PaletteEntry::Transparent {},
    ];
    let palette = identity::palette_content(&entries).unwrap();
    assert_eq!(palette.0, frozen::palette_identity(&entries).0);
    let input = [1, 2, 3, 4, 5, 6, 7, 8];
    let parent = source_identity(Source {
        width: 2,
        height: 1,
        data: &input,
    });
    assert_eq!(
        parent.0,
        frozen::source_identity(reference::Source {
            width: 2,
            height: 1,
            data: &input
        })
        .0
    );
    let spaces = [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ];
    for space in spaces {
        for strength in [-0.0, 0.1, 1.0, f32::MAX] {
            let perturb = PerturbPolicy {
                field: Field::Random { seed: 42 },
                space,
                strength,
                placement: Placement::Adaptive {
                    radius: 3,
                    threshold: -0.0,
                    softness: 0.1,
                },
            };
            let stages = [
                StageOptions::Color { space },
                StageOptions::Perturb { perturb },
                StageOptions::Alpha {
                    palette,
                    alpha: AlphaPolicy::Preserve {
                        threshold: 127.9999999,
                    },
                },
                StageOptions::Indexed {
                    palette,
                    alpha: AlphaPolicy::Preserve { threshold: -0.0 },
                    matching: MatchPolicy::OklchHueArc,
                    dither: DitherPolicy::Separable { perturb },
                },
                StageOptions::Indexed {
                    palette,
                    alpha: AlphaPolicy::Matte { rgb: [5, 4, 3] },
                    matching: MatchPolicy::SrgbRec709,
                    dither: DitherPolicy::Diffusion {
                        kernel: Diffusion::Sierra,
                        strength,
                        placement: perturb.placement,
                        serpentine: true,
                        feedback: DiffusionFeedback::Matching,
                    },
                },
                StageOptions::Indexed {
                    palette,
                    alpha: AlphaPolicy::Premultiplied {},
                    matching: MatchPolicy::CielabCiede2000,
                    dither: DitherPolicy::Yliluoma {
                        size: BayerSize::Eight,
                        placement: perturb.placement,
                    },
                },
            ];
            for stage in stages {
                assert_eq!(
                    identity::stage(Some(parent), stage).unwrap().0,
                    frozen::stage_identity(
                        Some(frozen::Identity(parent.0)),
                        1,
                        frozen_stage(stage)
                    )
                    .0
                );
            }
        }
    }
}

#[test]
fn no_dither_composition_matches_direct_and_fused_reference_plans() {
    let bytes = [19, 43, 217, 255];
    let source = Source {
        width: 1,
        height: 1,
        data: &bytes,
    };
    let entries = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let alpha = AlphaPolicy::Preserve { threshold: 0.0 };
    let matching = MatchPolicy::SrgbEuclidean;
    let actual = identity::indexed(
        source_identity(source),
        identity::palette_content(&entries).unwrap(),
        alpha,
        matching,
        DitherPolicy::None {},
    )
    .unwrap();
    let quantize = reference::QuantizeRequest {
        version: 1,
        source: reference::Source {
            width: 1,
            height: 1,
            data: &bytes,
        },
        palette: &entries,
        alpha: reference::AlphaPolicy::Preserve { threshold: 0.0 },
        matching: reference::MatchPolicy::SrgbEuclidean,
    };
    for request in [
        reference::Request::Quantize(quantize),
        reference::Request::DitherAndQuantize(reference::DitherQuantizeRequest {
            quantize,
            dither: reference::DitherPolicy::None {},
        }),
    ] {
        assert_eq!(
            actual.0,
            frozen::request_identity_plan(request)
                .unwrap()
                .resolve(&[])
                .unwrap()
                .final_identity
                .0
        );
    }
    assert_ne!(
        actual,
        identity::indexed(
            Identity([0; 32]),
            identity::palette_content(&entries).unwrap(),
            alpha,
            matching,
            DitherPolicy::None {}
        )
        .unwrap()
    );
}

#[test]
fn all_methods_share_keys_at_actual_reference_rgba_outputs() {
    let bytes = [10, 40, 60, 128, 200, 140, 90, 255];
    let source = reference::Source {
        width: 2,
        height: 1,
        data: &bytes,
    };
    let output = Output {
        width: 1,
        height: 2,
        resize: ResizePolicy::Area {},
    };
    let resize_request = reference::ResizeRequest {
        version: 1,
        source,
        output: serde_json::from_value(serde_json::to_value(output).unwrap()).unwrap(),
    };
    let resize = identity::stage(
        Some(Identity(frozen::source_identity(source).0)),
        StageOptions::Resize { output },
    )
    .unwrap();
    assert_eq!(
        resize.0,
        frozen::request_identity_plan(reference::Request::Resize(resize_request))
            .unwrap()
            .resolve(&[])
            .unwrap()
            .final_identity
            .0
    );
    let resized = ditherette_wasm::spec::resize::resize(resize_request).unwrap();
    let resized_source = reference::Source {
        width: 1,
        height: 2,
        data: resized.data(),
    };
    let resized_content = frozen::source_identity(resized_source);
    assert_ne!(resize.0, resized_content.0);
    let entries = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Transparent {},
    ];
    let palette = identity::palette_content(&entries).unwrap();
    let alpha = AlphaPolicy::Preserve {
        threshold: 127.9999999,
    };
    let perturb = PerturbPolicy {
        field: Field::Bayer {
            size: BayerSize::Two,
        },
        space: WorkingSpace::Srgb,
        strength: 1.0,
        placement: Placement::Everywhere {},
    };
    for dither in [
        DitherPolicy::None {},
        DitherPolicy::Separable { perturb },
        DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            strength: 1.0,
            placement: Placement::Everywhere {},
            serpentine: false,
            feedback: DiffusionFeedback::SrgbBytes,
        },
        DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    ] {
        let reference_dither =
            serde_json::from_value(serde_json::to_value(dither).unwrap()).unwrap();
        let quantize = reference::QuantizeRequest {
            version: 1,
            source: resized_source,
            palette: &entries,
            alpha: reference::AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            matching: reference::MatchPolicy::SrgbEuclidean,
        };
        let fused = frozen::request_identity_plan(reference::Request::DitherAndQuantize(
            reference::DitherQuantizeRequest {
                quantize,
                dither: reference_dither,
            },
        ))
        .unwrap();
        let process =
            frozen::request_identity_plan(reference::Request::Process(reference::ProcessRequest {
                source,
                palette: &entries,
                recipe: reference::RecipeV1 {
                    version: 1,
                    output: resize_request.output,
                    alpha: quantize.alpha,
                    matching: quantize.matching,
                    dither: reference_dither,
                },
            }))
            .unwrap();
        assert_eq!(resize.0, process.key_at(0, &[]).unwrap().0);
        let mut process_outputs = vec![(0, resized_content)];
        let mut fused_outputs = Vec::new();
        let (parent, indexed_dither) = if let DitherPolicy::Separable { perturb } = dither {
            let raw = identity::stage(
                Some(Identity(resized_content.0)),
                StageOptions::Color {
                    space: perturb.space,
                },
            )
            .unwrap();
            let actual = identity::stage(
                Some(Identity(resized_content.0)),
                StageOptions::Perturb { perturb },
            )
            .unwrap();
            let reference_perturb =
                serde_json::from_value(serde_json::to_value(perturb).unwrap()).unwrap();
            let standalone = frozen::request_identity_plan(reference::Request::Perturb(
                reference::PerturbRequest {
                    version: 1,
                    source: resized_source,
                    perturb: reference_perturb,
                },
            ))
            .unwrap()
            .resolve(&[])
            .unwrap();
            assert_eq!(raw.0, standalone.operations[0].0);
            assert_eq!(actual.0, standalone.final_identity.0);
            assert_eq!(actual.0, fused.key_at(1, &[]).unwrap().0);
            let perturbed = ditherette_wasm::spec::dither::perturb::perturb(
                resized.as_view(),
                reference_perturb,
            )
            .unwrap();
            let perturbed_source = reference::Source {
                width: 1,
                height: 2,
                data: perturbed.data(),
            };
            let content = frozen::source_identity(perturbed_source);
            process_outputs.push((2, content));
            fused_outputs.push((1, content));
            let direct = frozen::request_identity_plan(reference::Request::Quantize(
                reference::QuantizeRequest {
                    source: perturbed_source,
                    ..quantize
                },
            ))
            .unwrap()
            .resolve(&[])
            .unwrap();
            assert_eq!(
                direct.final_identity,
                fused.resolve(&fused_outputs).unwrap().final_identity
            );
            (Identity(content.0), DitherPolicy::None {})
        } else {
            (Identity(resized_content.0), dither)
        };
        let actual = identity::indexed(
            parent,
            palette,
            alpha,
            MatchPolicy::SrgbEuclidean,
            indexed_dither,
        )
        .unwrap();
        assert_eq!(
            actual.0,
            process.resolve(&process_outputs).unwrap().final_identity.0
        );
        assert_eq!(
            actual.0,
            fused.resolve(&fused_outputs).unwrap().final_identity.0
        );
    }
}
