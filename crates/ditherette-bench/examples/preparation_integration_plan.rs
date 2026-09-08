//! Generate the declared S31 matrix without timing any processing call.
use ditherette_bench::paired::{
    browser::{
        BrowserBackend, BrowserCase, BrowserPreparation, CacheCapability, PreparationCapability,
        PublicOperation,
    },
    coordinator::validate_experiment,
    native::NativeOperation,
    preparation::ProcessorSettings,
    process::ProcessSettings,
    quantize::QuantizeSettings,
    *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use ditherette_wasm::{image::contracts::PaletteEntry, spec::contract::request::*};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

pub fn palette(count: u16) -> Vec<PaletteEntry> {
    (0..count)
        .map(|n| PaletteEntry::Color {
            rgb: [n as u8, n.wrapping_mul(73) as u8, n.wrapping_mul(151) as u8],
        })
        .collect()
}

pub fn source_rgba(source: Dimensions) -> Vec<u8> {
    (0..source.width * source.height)
        .flat_map(|n| {
            let x = n % source.width;
            let y = n / source.width;
            [
                (x * 17 + y * 31) as u8,
                (x * 43 + y * 7) as u8,
                (x * 11 + y * 53) as u8,
                255,
            ]
        })
        .collect()
}

pub fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let output = |resize| Output {
        width: 65,
        height: 49,
        resize,
    };
    let cases = [
        (
            "quantize-lab76-p256",
            Dimensions {
                width: 32,
                height: 24,
            },
            ProcessorSettings::Quantize {
                settings: QuantizeSettings {
                    palette: palette(256),
                    alpha: ditherette_bench::paired::quantize::AlphaPolicy::Premultiplied {},
                    matching: ditherette_bench::paired::quantize::MatchPolicy::CielabEuclidean,
                },
            },
        ),
        (
            "resize-lanczos3",
            Dimensions {
                width: 129,
                height: 97,
            },
            ProcessorSettings::Resize {
                output: output(ResizePolicy::Lanczos3 {
                    anchor: Anchor::Center,
                    support: Support::ScaleAware,
                }),
            },
        ),
        (
            "resize-trilinear",
            Dimensions {
                width: 256,
                height: 192,
            },
            ProcessorSettings::Resize {
                output: output(ResizePolicy::Trilinear {
                    anchor: Anchor::Center,
                }),
            },
        ),
        (
            "process-lanczos2-floyd-steinberg-p16",
            Dimensions {
                width: 129,
                height: 97,
            },
            ProcessorSettings::Process {
                settings: ProcessSettings {
                    palette: palette(16),
                    recipe: RecipeV1 {
                        version: 1,
                        output: output(ResizePolicy::Lanczos2 {
                            anchor: Anchor::Center,
                            support: Support::ScaleAware,
                        }),
                        alpha: AlphaPolicy::Premultiplied {},
                        matching: MatchPolicy::SrgbEuclidean,
                        dither: DitherPolicy::Diffusion {
                            kernel: Diffusion::FloydSteinberg,
                            feedback: DiffusionFeedback::SrgbBytes,
                            strength: 0.7,
                            serpentine: true,
                            placement: Placement::Everywhere {},
                        },
                    },
                },
            },
        ),
    ];
    let cache = CacheCapability::Roles {
        accepted: PreparationCapability::Uncached,
        candidate: PreparationCapability::Preparation,
        sample_prime: None,
    };
    let mut matrix = Vec::new();
    for (name, source, settings) in cases {
        let rgba = source_rgba(source);
        let native = NativeOperation::Processor {
            settings: settings.clone(),
            cache,
        };
        let operation = match &settings {
            ProcessorSettings::Quantize { settings } => PublicOperation::Quantize {
                settings: settings.clone(),
            },
            ProcessorSettings::Process { settings } => PublicOperation::Process {
                settings: settings.clone(),
            },
            ProcessorSettings::Separable { settings } => PublicOperation::Separable {
                settings: settings.clone(),
            },
            ProcessorSettings::Resize { output } => match output.resize {
                ResizePolicy::Lanczos3 { .. } => PublicOperation::ResizeLanczos3 {
                    anchor: browser::Anchor::Center,
                    support: browser::Support::ScaleAware,
                },
                ResizePolicy::Trilinear { .. } => PublicOperation::ResizeTrilinear {
                    anchor: browser::Anchor::Center,
                },
                _ => unreachable!("declared resize matrix"),
            },
        };
        let identity = native.identity(source, &rgba)?;
        for application_cache in [ApplicationCache::Cold, ApplicationCache::Warm] {
            let cold = application_cache == ApplicationCache::Cold;
            let reference_subject = if public {
                operation.reference_subject()
            } else {
                native.reference_subject()
            };
            let subject = if public {
                operation.subject(BrowserBackend::Package)
            } else {
                settings.subject()
            };
            matrix.push(PairCase {
                name: format!(
                    "{name}-{}x{}-{}",
                    source.width,
                    source.height,
                    if cold { "cold" } else { "warm" }
                ),
                source,
                rgba: rgba.clone(),
                identity: if public {
                    operation.identity(source, &rgba, identity.output)?
                } else {
                    identity.clone()
                },
                reference_subject: reference_subject.into(),
                accepted_subject: subject.into(),
                candidate_subject: subject.into(),
                native: (!public).then_some(native.clone()),
                browser: public.then_some(BrowserCase {
                    operation: operation.clone(),
                    accepted: BrowserBackend::Package,
                    candidate: BrowserBackend::Package,
                    preparation: if cold {
                        BrowserPreparation::FreshInstance
                    } else {
                        BrowserPreparation::PrimedInstance
                    },
                    cache,
                    measure_nonexact: false,
                }),
                measurement: Measurement {
                    mode: SampleMode::SingleCall,
                    scope: if public {
                        CallScope::CompleteCall
                    } else {
                        CallScope::NativeCompleteCall
                    },
                    application_cache,
                    samples: 20,
                    measurement_ms: 10_000,
                    warmup_ms: 50,
                    target_sample_ms: 1,
                },
            });
        }
    }
    let experiment = Experiment {
        label: "preparation-reuse".into(),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes: notes,
        cases: matrix,
    };
    validate_experiment(&experiment)?;
    Ok(experiment)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [kind, destination, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: preparation_integration_plan native|public DESTINATION HOST_LOAD_NOTES",
        ));
    };
    if !matches!(kind.as_str(), "native" | "public") {
        return Err(io::Error::other("unknown plan kind"));
    }
    let plan = experiment(kind == "public", notes.clone())?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_matrix_is_single_call_and_binds_measured_bytes() {
        for public in [false, true] {
            let plan = experiment(public, "untimed fixture validation".into()).unwrap();
            assert_eq!(plan.cases.len(), 8);
            assert_eq!(plan.pairs, 2);
            for case in plan.cases {
                assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                assert_eq!(case.measurement.samples, 20);
                assert_eq!(
                    case.identity.input,
                    ditherette_bench::verification::input_digest(case.source, &case.rgba)
                );
                let mut invalid = case.clone();
                invalid.measurement.mode = SampleMode::Throughput;
                assert!(browser::validate_case(&invalid).is_err());
            }
        }
    }

    #[test]
    fn processor_subjects_match_frozen_outputs_after_changed_source_prime() {
        use ditherette_wasm::bench_subjects::{
            self, field_calls, preparation::CompleteCall, BenchSubject,
        };
        let registry = bench_subjects::bench_subjects();
        for case in experiment(false, "untimed conformance".into())
            .unwrap()
            .cases
            .into_iter()
            .step_by(2)
        {
            let operation = case.native.as_ref().unwrap();
            let request = operation
                .reference_request(case.source, &case.rgba)
                .unwrap();
            let frozen = registry
                .iter()
                .find_map(|subject| match subject {
                    BenchSubject::Conformance(subject)
                        if subject.descriptor.id.as_str() == case.reference_subject =>
                    {
                        Some((subject.run)(&request).unwrap())
                    }
                    _ => None,
                })
                .unwrap();
            let call = CompleteCall::new(&request).unwrap();
            let cold = call
                .output(&mut field_calls::processor().unwrap(), &case.rgba)
                .unwrap()
                .verification();
            let mut warm = field_calls::processor().unwrap();
            let mut prime = case.rgba.clone();
            for pixel in prime.chunks_exact_mut(4) {
                pixel[0] ^= 0xff;
            }
            drop(std::hint::black_box(
                call.output(&mut warm, &prime).unwrap(),
            ));
            assert_eq!(cold, frozen, "{} cold", case.name);
            assert_eq!(
                call.output(&mut warm, &case.rgba).unwrap().verification(),
                frozen,
                "{} warm",
                case.name
            );
            let mut invalid = case.clone();
            invalid.measurement.application_cache = ApplicationCache::NotApplicable;
            assert!(browser::validate_case(&invalid).is_err());
        }
    }
}
