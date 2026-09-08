//! Declares S24's 124-worker ceiling. This executable never measures operations.
use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, native::*, quantize::*, *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let source = Dimensions {
        width: 128,
        height: 96,
    };
    let rgba: Vec<_> = (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                [
                    (x * 17 + y * 7 + 11) as u8,
                    (y * 31 + x * 3 + 23) as u8,
                    ((x ^ y) + 47) as u8,
                    (x * 43 + y * 19) as u8,
                ]
            })
        })
        .collect();
    let spaces = [
        ("srgb", WorkingSpace::Srgb, MatchPolicy::SrgbEuclidean),
        (
            "linear-rgb",
            WorkingSpace::LinearRgb,
            MatchPolicy::LinearRgbEuclidean,
        ),
        ("oklab", WorkingSpace::Oklab, MatchPolicy::OklabEuclidean),
        ("cielab", WorkingSpace::Cielab, MatchPolicy::CielabEuclidean),
        ("ycbcr", WorkingSpace::Ycbcr, MatchPolicy::YcbcrEuclidean),
    ];
    let mut recipes = Vec::new();
    for (name, space, matching) in spaces {
        if !public {
            recipes.push((
                format!("{name}-forward"),
                NativeOperation::ColorForward { space },
                SampleMode::SingleCall,
            ));
        }
        let mut variants = vec![(64, SampleMode::SingleCall)];
        if matching == MatchPolicy::SrgbEuclidean {
            if !public {
                variants.extend([(16, SampleMode::SingleCall), (256, SampleMode::SingleCall)]);
            }
            variants.push((64, SampleMode::Throughput));
        }
        for (count, mode) in variants {
            let mut palette: Vec<_> = (0..count - 1)
                .map(|i| PaletteEntry::Color {
                    rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
                })
                .collect();
            palette.push(PaletteEntry::Transparent {});
            recipes.push((
                format!(
                    "{name}-palette{count}-{}",
                    if mode == SampleMode::SingleCall {
                        "latency"
                    } else {
                        "throughput"
                    }
                ),
                NativeOperation::Quantize {
                    settings: QuantizeSettings {
                        palette,
                        alpha: AlphaPolicy::Preserve { threshold: 0.5 },
                        matching,
                    },
                },
                mode,
            ));
        }
    }
    let mut cases = Vec::new();
    for (name, native, mode) in recipes {
        let (accepted, candidate, browser) = if public {
            let NativeOperation::Quantize { settings } = &native else {
                unreachable!()
            };
            let operation = PublicOperation::Quantize {
                settings: settings.clone(),
            };
            let subject = operation.subject(BrowserBackend::Package).to_owned();
            (
                subject.clone(),
                subject,
                Some(BrowserCase {
                    operation,
                    accepted: BrowserBackend::Package,
                    candidate: BrowserBackend::Package,
                    preparation: BrowserPreparation::PrimedInstance,
                    cache: CacheCapability::None,
                    measure_nonexact: false,
                    progress: None,
                }),
            )
        } else {
            match &native {
                NativeOperation::Processor { .. } => unreachable!("quantize integration cases"),
                NativeOperation::Diffusion { .. }
                | NativeOperation::Process { .. }
                | NativeOperation::MetricScores { .. }
                | NativeOperation::Yliluoma { .. }
                | NativeOperation::FieldComponent { .. }
                | NativeOperation::Perturb { .. }
                | NativeOperation::Separable { .. } => {
                    unreachable!("S24 has no metric or field controls")
                }
                NativeOperation::Quantize { .. } => (
                    "baseline:quantize:request:literal".into(),
                    "candidate:quantize:request:prepared".into(),
                    None,
                ),
                NativeOperation::ColorForward { space } => {
                    let id = ditherette_wasm::bench_subjects::quantize::color_subject(*space)
                        .map_err(io::Error::other)?
                        .to_owned();
                    (id.clone(), id, None)
                }
            }
        };
        cases.push(PairCase {
            name,
            identity: native.identity(source, &rgba)?,
            source,
            rgba: rgba.clone(),
            reference_subject: native.reference_subject().into(),
            accepted_subject: accepted,
            candidate_subject: candidate,
            measurement: Measurement {
                mode,
                scope: if public {
                    CallScope::CompleteCall
                } else {
                    native.scope()
                },
                application_cache: ApplicationCache::NotApplicable,
                samples: 20,
                warmup_ms: 50,
                measurement_ms: 250,
                target_sample_ms: 2,
            },
            browser,
            native: (!public).then_some(native),
        });
    }
    let experiment = Experiment {
        label: if public {
            "S24 public package controls"
        } else {
            "S24 literal versus prepared native controls"
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
            "usage: quantize_integration_plan native|public output.json host-load-notes",
        ));
    };
    let public = match kind.as_str() {
        "native" => false,
        "public" => true,
        _ => return Err(io::Error::other("kind must be native or public")),
    };
    let plan = experiment(public, notes.clone())?;
    let bytes = serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?
        .write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_budget_and_scopes_are_declared_without_measurement() {
        let native = experiment(false, "fixture".into()).unwrap();
        let public = experiment(true, "fixture".into()).unwrap();
        assert_eq!(native.cases.len(), 13);
        assert_eq!(public.cases.len(), 6);
        assert_eq!((native.cases.len() + public.cases.len() * 3) * 2 * 2, 124);
        assert_eq!(
            native
                .cases
                .iter()
                .filter(|c| c.measurement.scope == CallScope::NativeForwardConversion)
                .count(),
            5
        );
        assert!(public
            .cases
            .iter()
            .all(|c| c.native.is_none() && c.accepted_subject == c.candidate_subject));
    }
}
