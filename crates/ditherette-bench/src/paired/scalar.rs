//! Bounded native scalar coverage. Declaration performs no measurements.
//!
//! Components exclude caller-owned storage, conversion fixtures, and verifier rendering.
//! Complete quantize/diffusion/Yliluoma calls borrow RGBA8 and include validation,
//! preparation, scratch, owned results, and destruction. Application caches do not apply.
//! A spec/prod ratio describes the current gap; only prod/prod tests an optimization regression.

use super::{diffusion::DiffusionSettings, native::*, quantize::*, yliluoma::YliluomaSettings, *};
use ditherette_wasm::{
    bench_subjects::{diffusion, fields::Component, quantize, scalar, yiluoma},
    spec::contract::request as spec,
};
use std::io;

/// The same cases compare frozen semantics with production or two production revisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    SpecProd,
    ProdProd,
}

fn fixture(source: Dimensions) -> Vec<u8> {
    (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                [
                    (x * 17 + y * 7 + 11) as u8,
                    (y * 31 + x * 3 + 23) as u8,
                    ((x ^ y) + 47) as u8,
                    (x * 43 + y * 19 + 255) as u8,
                ]
            })
        })
        .collect()
}

fn measurement(scope: CallScope) -> Measurement {
    Measurement {
        mode: SampleMode::SingleCall,
        scope,
        application_cache: ApplicationCache::NotApplicable,
        samples: 20,
        measurement_ms: 250,
        warmup_ms: 50,
        target_sample_ms: 2,
    }
}

fn quantize(matching: MatchPolicy, size: u32) -> QuantizeSettings {
    let mut palette: Vec<_> = (0..size - 1)
        .map(|i| PaletteEntry::Color {
            rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
        })
        .collect();
    palette.push(PaletteEntry::Transparent {});
    QuantizeSettings {
        palette,
        alpha: AlphaPolicy::Preserve { threshold: 0.5 },
        matching,
    }
}

fn tag(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .expect("enum serialization")
        .as_str()
        .expect("enum tag")
        .into()
}

fn production(operation: &NativeOperation) -> &str {
    match operation {
        NativeOperation::ColorForward { space } => {
            quantize::color_subject(*space).expect("color space")
        }
        NativeOperation::MetricScores { metric } => metric.prod_subject(),
        NativeOperation::FieldComponent { component } => component.prod_subject(),
        NativeOperation::PerturbComponent { .. } => scalar::PERTURB_SUBJECT,
        NativeOperation::Quantize { .. } => quantize::QUANTIZE_SUBJECT,
        NativeOperation::Diffusion { .. } => diffusion::CANDIDATE_SUBJECT,
        NativeOperation::Yliluoma { .. } => yiluoma::YLILUOMA_SUBJECT,
        _ => unreachable!("bounded scalar operations"),
    }
}

fn typed_case(
    name: String,
    operation: NativeOperation,
    source: Dimensions,
) -> io::Result<PairCase> {
    let rgba = fixture(source);
    Ok(PairCase {
        name,
        identity: operation.identity(source, &rgba)?,
        source,
        rgba,
        reference_subject: operation.reference_subject().into(),
        accepted_subject: operation.reference_subject().into(),
        candidate_subject: production(&operation).into(),
        measurement: measurement(operation.scope()),
        browser: None,
        native: Some(operation),
    })
}

/// All current scalar families, representative images, and selected tiny setup controls.
/// Two alternating pairs cap each case at four sequential workers and 80 retained samples.
pub fn experiment(comparison: Comparison, host_load_notes: String) -> io::Result<Experiment> {
    let common = Dimensions {
        width: 128,
        height: 96,
    };
    let component = Dimensions {
        width: 512,
        height: 384,
    };
    let mut cases = Vec::new();
    for (filter, variant) in [
        ("nearest", "scalar"),
        ("area", "scalar"),
        ("bilinear", "scalar"),
        ("bicubic", "catmull-rom"),
        ("bicubic", "catmull-rom-scale-aware"),
        ("lanczos2", "fixed"),
        ("lanczos2", "scale-aware"),
        ("lanczos3", "fixed"),
        ("lanczos3", "scale-aware"),
        ("trilinear", "mip-area"),
    ] {
        let reference = format!("spec:resize:{filter}:{variant}");
        let rgba = fixture(component);
        let output = Dimensions {
            width: 173,
            height: 129,
        };
        cases.push(PairCase {
            name: format!("resize-{filter}-{variant}"),
            identity: identity(&reference, component, &rgba, output)?,
            source: component,
            rgba,
            reference_subject: reference.clone(),
            accepted_subject: reference,
            candidate_subject: format!("prod:resize:{filter}:{variant}"),
            measurement: measurement(CallScope::NativeKernel),
            browser: None,
            native: None,
        });
    }
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Cielab,
        WorkingSpace::Ycbcr,
        WorkingSpace::Oklch,
        WorkingSpace::Cielch,
    ] {
        cases.push(typed_case(
            format!("forward-{}", tag(space)),
            NativeOperation::ColorForward { space },
            component,
        )?);
        cases.push(typed_case(
            format!("inverse-{}", tag(space)),
            NativeOperation::FieldComponent {
                component: Component::Inverse { space },
            },
            component,
        )?);
        cases.push(typed_case(
            format!("source-construction-{}", tag(space)),
            NativeOperation::FieldComponent {
                component: Component::SourceConversion { space },
            },
            common,
        )?);
        cases.push(typed_case(
            format!("reconstruct-{}", tag(space)),
            NativeOperation::FieldComponent {
                component: Component::Reconstruct { space },
            },
            component,
        )?);
        cases.push(typed_case(
            format!("placement-{}", tag(space)),
            NativeOperation::FieldComponent {
                component: Component::Placement {
                    space,
                    placement: adaptive(),
                },
            },
            common,
        )?);
    }
    for metric in MetricFamily::ALL {
        cases.push(typed_case(
            format!("scores-{}", tag(metric)),
            NativeOperation::MetricScores { metric },
            component,
        )?);
    }
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::YcbcrEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
    ] {
        cases.push(typed_case(
            format!("quantize-{}-palette32", tag(matching)),
            NativeOperation::Quantize {
                settings: quantize(matching, 32),
            },
            common,
        )?);
    }
    for (name, field, space, placement) in [
        (
            "bayer2",
            spec::Field::Bayer {
                size: spec::BayerSize::Two,
            },
            WorkingSpace::Srgb,
            spec::Placement::Everywhere {},
        ),
        (
            "bayer4",
            spec::Field::Bayer {
                size: spec::BayerSize::Four,
            },
            WorkingSpace::LinearRgb,
            spec::Placement::Everywhere {},
        ),
        (
            "bayer8",
            spec::Field::Bayer {
                size: spec::BayerSize::Eight,
            },
            WorkingSpace::Oklab,
            adaptive(),
        ),
        (
            "bayer16",
            spec::Field::Bayer {
                size: spec::BayerSize::Sixteen,
            },
            WorkingSpace::Cielab,
            spec::Placement::Everywhere {},
        ),
        (
            "random",
            spec::Field::Random { seed: 0x12345678 },
            WorkingSpace::Oklch,
            adaptive(),
        ),
        (
            "blue",
            spec::Field::BlueNoise {},
            WorkingSpace::Cielch,
            spec::Placement::Everywhere {},
        ),
    ] {
        cases.push(typed_case(
            format!("threshold-{name}"),
            NativeOperation::FieldComponent {
                component: Component::Field { field },
            },
            component,
        )?);
        cases.push(typed_case(
            format!("perturb-{name}-{}", tag(space)),
            NativeOperation::PerturbComponent {
                settings: spec::PerturbPolicy {
                    field,
                    space,
                    strength: 0.7,
                    placement,
                },
            },
            common,
        )?);
    }
    for kernel in [
        spec::Diffusion::FloydSteinberg,
        spec::Diffusion::Sierra,
        spec::Diffusion::SierraLite,
        spec::Diffusion::Atkinson,
    ] {
        for feedback in [
            spec::DiffusionFeedback::SrgbBytes,
            spec::DiffusionFeedback::Matching,
        ] {
            cases.push(typed_case(
                format!("diffusion-{}-{}", tag(kernel), tag(feedback)),
                NativeOperation::Diffusion {
                    settings: DiffusionSettings {
                        quantize: quantize(MatchPolicy::OklabEuclidean, 32),
                        kernel,
                        feedback,
                        strength: 0.7,
                        serpentine: true,
                        placement: spec::Placement::Everywhere {},
                    },
                },
                common,
            )?);
        }
    }
    for size in [
        spec::BayerSize::Two,
        spec::BayerSize::Four,
        spec::BayerSize::Eight,
        spec::BayerSize::Sixteen,
    ] {
        cases.push(typed_case(
            format!("yliluoma-{}", tag(size)),
            NativeOperation::Yliluoma {
                settings: YliluomaSettings {
                    quantize: quantize(MatchPolicy::SrgbEuclidean, 8),
                    size,
                    placement: spec::Placement::Everywhere {},
                },
            },
            Dimensions {
                width: 32,
                height: 24,
            },
        )?);
    }
    let mut zero = cases
        .iter()
        .find(|case| case.name == "perturb-bayer4-linear-rgb")
        .expect("perturb case")
        .clone();
    let Some(NativeOperation::PerturbComponent { settings }) = &mut zero.native else {
        unreachable!()
    };
    settings.strength = 0.0;
    zero.name = "perturb-zero-strength".into();
    zero.identity = zero
        .native
        .as_ref()
        .unwrap()
        .identity(zero.source, &zero.rgba)?;
    cases.push(zero);
    for prefix in [
        "resize-nearest",
        "forward-srgb",
        "inverse-srgb",
        "scores-euclidean",
        "quantize-srgb-euclidean",
        "threshold-random",
        "diffusion-floyd-steinberg-srgb-bytes",
        "yliluoma-",
        "perturb-bayer4",
    ] {
        let original = cases
            .iter()
            .find(|case| case.name.starts_with(prefix))
            .expect("control case");
        let tiny = Dimensions {
            width: 1,
            height: 1,
        };
        let control = if let Some(operation) = &original.native {
            typed_case(format!("tiny-{}", original.name), operation.clone(), tiny)?
        } else {
            let mut case = original.clone();
            case.name = format!("tiny-{}", case.name);
            case.source = tiny;
            case.rgba = fixture(tiny);
            case.identity = identity(
                &case.reference_subject,
                tiny,
                &case.rgba,
                Dimensions {
                    width: 2,
                    height: 2,
                },
            )?;
            case
        };
        cases.push(control);
    }
    if comparison == Comparison::ProdProd {
        for case in &mut cases {
            case.accepted_subject = case.candidate_subject.clone();
        }
    }
    let experiment = Experiment {
        label: format!("scalar {comparison:?}; matched components and borrowed complete calls; resize drift diagnostic only"),
        reference_state: ReferenceState::Frozen, pairs: 2, host_load_notes, cases,
    };
    coordinator::validate_experiment(&experiment)?;
    Ok(experiment)
}

fn adaptive() -> spec::Placement {
    spec::Placement::Adaptive {
        radius: 1,
        threshold: 10.0,
        softness: 5.0,
    }
}
