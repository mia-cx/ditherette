//! Declares S28's bounded comparison. This executable never measures operations.

use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, diffusion::DiffusionSettings,
    native::NativeOperation, quantize::*, *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use ditherette_wasm::{
    bench_subjects::diffusion,
    spec::contract::request::{Diffusion, DiffusionFeedback, Placement},
};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let source = Dimensions {
        width: 65,
        height: 33,
    };
    let rgba: Vec<_> = (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                [
                    (x * 17 + y * 7 + 11) as u8,
                    (y * 31 + x * 3 + 23) as u8,
                    ((x ^ y) * 13 + 47) as u8,
                    match (x + y) % 5 {
                        0 => 0,
                        1 => 255,
                        _ => (x * 43 + y * 19) as u8,
                    },
                ]
            })
        })
        .collect();
    let mut palette: Vec<_> = (0..15)
        .map(|i| PaletteEntry::Color {
            rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
        })
        .collect();
    palette.push(PaletteEntry::Transparent {});

    let mut cases = Vec::new();
    for (kernel_index, kernel) in [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
    ]
    .into_iter()
    .enumerate()
    {
        for (feedback_index, feedback) in
            [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching]
                .into_iter()
                .enumerate()
        {
            let index = kernel_index * 2 + feedback_index;
            let settings = DiffusionSettings {
                quantize: QuantizeSettings {
                    palette: palette.clone(),
                    matching: [
                        MatchPolicy::SrgbEuclidean,
                        MatchPolicy::OklabEuclidean,
                        MatchPolicy::CielabCiede2000,
                    ][index % 3],
                    alpha: [
                        AlphaPolicy::Preserve { threshold: 0.5 },
                        AlphaPolicy::Premultiplied {},
                        AlphaPolicy::Matte { rgb: [17, 83, 149] },
                    ][(index + 1) % 3],
                },
                kernel,
                feedback,
                strength: 0.7,
                serpentine: (kernel_index + feedback_index) % 2 == 1,
                placement: if index % 3 == 0 {
                    Placement::Everywhere {}
                } else {
                    Placement::Adaptive {
                        radius: 1 + (index % 2) as u32,
                        threshold: 10.0,
                        softness: 5.0,
                    }
                },
            };
            let native = NativeOperation::Diffusion {
                settings: settings.clone(),
            };
            let operation = PublicOperation::Diffusion { settings };
            let subject = operation.subject(BrowserBackend::Package);
            let name = format!(
                "diffusion-{}-{}-65x33-palette16",
                serde_json::to_value(kernel)
                    .map_err(io::Error::other)?
                    .as_str()
                    .unwrap(),
                serde_json::to_value(feedback)
                    .map_err(io::Error::other)?
                    .as_str()
                    .unwrap(),
            );
            cases.push(PairCase {
                name,
                identity: native.identity(source, &rgba)?,
                source,
                rgba: rgba.clone(),
                reference_subject: native.reference_subject().into(),
                accepted_subject: if public { subject } else { diffusion::SUBJECT }.into(),
                candidate_subject: if public {
                    subject
                } else {
                    diffusion::CANDIDATE_SUBJECT
                }
                .into(),
                measurement: Measurement {
                    mode: SampleMode::SingleCall,
                    scope: if public {
                        CallScope::CompleteCall
                    } else {
                        CallScope::NativeCompleteCall
                    },
                    application_cache: ApplicationCache::NotApplicable,
                    samples: 20,
                    warmup_ms: 50,
                    measurement_ms: 10_000,
                    target_sample_ms: 2,
                },
                browser: public.then_some(BrowserCase {
                    operation,
                    accepted: BrowserBackend::Package,
                    candidate: BrowserBackend::Package,
                    preparation: BrowserPreparation::PrimedInstance,
                    cache: CacheCapability::None,
                    measure_nonexact: false,
                    progress: None,
                }),
                native: (!public).then_some(native),
            });
        }
    }
    let experiment = Experiment {
        label: if public {
            "S28 public ring self-pair, no full-image public comparison"
        } else {
            "S28 native borrowed-source literal full-image versus three-row scratch"
        }
        .into(),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes: notes,
        cases,
    };
    validate_experiment(&experiment)?;
    Ok(experiment)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [kind, output, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: diffusion_integration_plan native|public output.json host-load-notes",
        ));
    };
    let public = match kind.as_str() {
        "native" => false,
        "public" => true,
        _ => return Err(io::Error::other("kind must be native or public")),
    };
    let bytes =
        serde_json::to_vec_pretty(&experiment(public, notes.clone())?).map_err(io::Error::other)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn fixed_128_workers_cover_controls_without_a_timed_cartesian_product() {
        let native = experiment(false, "fixture".into()).unwrap();
        let public = experiment(true, "fixture".into()).unwrap();
        assert_eq!((native.cases.len(), public.cases.len()), (8, 8));
        assert_eq!(
            (native.cases.len() + public.cases.len() * 3) * native.pairs * 2,
            128
        );
        let mut controls: [HashSet<String>; 6] = std::array::from_fn(|_| HashSet::new());
        for (native, public) in native.cases.iter().zip(&public.cases) {
            assert_eq!(native.identity, public.identity);
            assert_eq!(native.rgba, public.rgba);
            assert_eq!(native.name, public.name);
            assert_eq!(native.accepted_subject, diffusion::SUBJECT);
            assert_eq!(native.candidate_subject, diffusion::CANDIDATE_SUBJECT);
            assert_eq!(public.accepted_subject, public.candidate_subject);
            assert_eq!(native.measurement.scope, CallScope::NativeCompleteCall);
            assert_eq!(public.measurement.scope, CallScope::CompleteCall);
            let Some(NativeOperation::Diffusion { settings }) = &native.native else {
                panic!("diffusion")
            };
            for (set, value) in controls.iter_mut().zip([
                serde_json::to_value(settings.kernel).unwrap(),
                serde_json::to_value(settings.feedback).unwrap(),
                serde_json::to_value(settings.quantize.matching).unwrap(),
                serde_json::to_value(settings.quantize.alpha).unwrap(),
                serde_json::to_value(settings.serpentine).unwrap(),
                serde_json::to_value(settings.placement).unwrap(),
            ]) {
                set.insert(value.to_string());
            }
            assert_eq!(settings.quantize.palette.len(), 16);
            for case in [native, public] {
                assert_eq!(
                    case.source,
                    Dimensions {
                        width: 65,
                        height: 33
                    }
                );
                assert_eq!(case.measurement.samples, 20);
                assert_eq!(case.measurement.warmup_ms, 50);
                assert_eq!(case.measurement.measurement_ms, 10_000);
                assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                assert_eq!(
                    case.measurement.application_cache,
                    ApplicationCache::NotApplicable
                );
            }
        }
        assert_eq!(controls.map(|set| set.len()), [4, 2, 3, 3, 2, 3]);
        for plan in [native, public] {
            assert_eq!(plan.pairs, 2);
            let decoded: Experiment =
                serde_json::from_slice(&serde_json::to_vec(&plan).unwrap()).unwrap();
            validate_experiment(&decoded).unwrap();
        }
    }
}
