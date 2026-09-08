//! Declares S25's 276-worker ceiling. This executable never measures operations.
use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, native::*, quantize::*, *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, existing: bool, notes: String) -> io::Result<Experiment> {
    // Identical to S24's declared varied-RGBA 128×96 fixture and 64-entry palette.
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
    let mut palette: Vec<_> = (0..63)
        .map(|i| PaletteEntry::Color {
            rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
        })
        .collect();
    palette.push(PaletteEntry::Transparent {});
    let existing_modes = [
        ("srgb-euclidean", MatchPolicy::SrgbEuclidean),
        ("linear-rgb-euclidean", MatchPolicy::LinearRgbEuclidean),
        ("oklab-euclidean", MatchPolicy::OklabEuclidean),
        ("cielab-euclidean", MatchPolicy::CielabEuclidean),
        ("ycbcr-euclidean", MatchPolicy::YcbcrEuclidean),
    ];
    let new_modes = [
        ("srgb-compuphase", MatchPolicy::SrgbCompuphase),
        ("srgb-rec601", MatchPolicy::SrgbRec601),
        ("srgb-rec709", MatchPolicy::SrgbRec709),
        ("oklch-euclidean", MatchPolicy::OklchEuclidean),
        ("oklch-circular-hue", MatchPolicy::OklchCircularHue),
        ("oklch-hue-arc", MatchPolicy::OklchHueArc),
        ("cielab-ciede2000", MatchPolicy::CielabCiede2000),
        ("cielch-euclidean", MatchPolicy::CielchEuclidean),
        ("cielch-circular-hue", MatchPolicy::CielchCircularHue),
        ("cielch-hue-arc", MatchPolicy::CielchHueArc),
    ];
    let modes = if existing {
        &existing_modes[..]
    } else {
        &new_modes[..]
    };
    let mut operations: Vec<_> = modes
        .iter()
        .map(|(name, matching)| {
            (
                format!("{name}-palette64-latency"),
                NativeOperation::Quantize {
                    settings: QuantizeSettings {
                        palette: palette.clone(),
                        alpha: AlphaPolicy::Preserve { threshold: 0.5 },
                        matching: *matching,
                    },
                },
            )
        })
        .collect();
    if !public && !existing {
        operations.extend([
            (
                "oklch-forward".into(),
                NativeOperation::ColorForward {
                    space: WorkingSpace::Oklch,
                },
            ),
            (
                "cielch-forward".into(),
                NativeOperation::ColorForward {
                    space: WorkingSpace::Cielch,
                },
            ),
        ]);
        operations.extend(MetricFamily::ALL.map(|metric| {
            (
                format!(
                    "{}-score-batch",
                    serde_json::to_value(metric)
                        .expect("enum")
                        .as_str()
                        .expect("tag")
                ),
                NativeOperation::MetricScores { metric },
            )
        }));
    }
    let mut cases = Vec::new();
    for (name, native) in operations {
        // Full CIEDE2000 scans 64 palette entries per pixel. Allow the minimum valid sample count.
        // This predeclared exception is not a speed claim or a post-measurement retry.
        let measurement_ms = if matches!(&native, NativeOperation::Quantize { settings }
            if settings.matching == MatchPolicy::CielabCiede2000)
        {
            10_000
        } else {
            250
        };
        let (subject, browser) = if public {
            let NativeOperation::Quantize { settings } = &native else {
                unreachable!()
            };
            let operation = PublicOperation::Quantize {
                settings: settings.clone(),
            };
            (
                operation.subject(BrowserBackend::Package).to_owned(),
                Some(BrowserCase {
                    operation,
                    accepted: BrowserBackend::Package,
                    candidate: BrowserBackend::Package,
                    preparation: BrowserPreparation::PrimedInstance,
                    cache: CacheCapability::None,
                    measure_nonexact: false,
                }),
            )
        } else {
            (
                match &native {
                    NativeOperation::Quantize { .. } => {
                        ditherette_wasm::bench_subjects::quantize::QUANTIZE_SUBJECT.into()
                    }
                    NativeOperation::ColorForward { space } => {
                        ditherette_wasm::bench_subjects::quantize::color_subject(*space)
                            .map_err(io::Error::other)?
                            .into()
                    }
                    NativeOperation::MetricScores { metric } => metric.prod_subject().into(),
                    NativeOperation::Diffusion { .. }
                    | NativeOperation::FieldComponent { .. }
                    | NativeOperation::Perturb { .. }
                    | NativeOperation::Separable { .. } => {
                        unreachable!("S25 has no field controls")
                    }
                },
                None,
            )
        };
        cases.push(PairCase {
            name,
            identity: native.identity(source, &rgba)?,
            source,
            rgba: rgba.clone(),
            reference_subject: native.reference_subject().into(),
            // Artifact identity, not a relabeled function, distinguishes accepted and candidate.
            accepted_subject: subject.clone(),
            candidate_subject: subject,
            measurement: Measurement {
                mode: SampleMode::SingleCall,
                scope: if public {
                    CallScope::CompleteCall
                } else {
                    native.scope()
                },
                application_cache: ApplicationCache::NotApplicable,
                samples: 20,
                warmup_ms: 50,
                measurement_ms,
                target_sample_ms: 2,
            },
            browser,
            native: (!public).then_some(native),
        });
    }
    let experiment = Experiment {
        label: format!(
            "S25 {} {} against dispatch candidate",
            if existing {
                "delivered S24 parent controls"
            } else {
                "0085972a all-mode baseline controls"
            },
            if public { "public" } else { "native" }
        ),
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
        return Err(io::Error::other("usage: matching_integration_plan existing-native|existing-public|new-native|new-public output.json host-load-notes"));
    };
    let (public, existing) = match kind.as_str() {
        "existing-native" => (false, true),
        "existing-public" => (true, true),
        "new-native" => (false, false),
        "new-public" => (true, false),
        _ => return Err(io::Error::other("unknown S25 experiment kind")),
    };
    let plan = experiment(public, existing, notes.clone())?;
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

    /// Reads existing evidence only. It neither runs a worker nor reuses its samples.
    #[test]
    #[ignore = "requires DITHERETTE_S24_RETAINED_ROOT from the coordinator"]
    fn existing_modes_roundtrip_actual_s24_requests_and_results_unchanged() {
        let root = std::path::PathBuf::from(
            env::var_os("DITHERETTE_S24_RETAINED_ROOT").expect("retained artifact root"),
        );
        for kind in ["native", "chromium", "firefox", "webkit"] {
            let public = kind != "native";
            let prepared = root.join(format!(
                "prepared-{kind}/{}prepared.json",
                if public { "pair/" } else { "" }
            ));
            let old: PreparedPair =
                serde_json::from_slice(&std::fs::read(prepared).unwrap()).unwrap();
            let new = experiment(public, true, "wire compatibility only".into()).unwrap();
            for case in &new.cases {
                let (index, previous) = old
                    .experiment
                    .cases
                    .iter()
                    .enumerate()
                    .find(|(_, old)| {
                        old.identity == case.identity && old.measurement == case.measurement
                    })
                    .expect("same S24 fixture/settings/timing scope");
                assert_eq!(previous.rgba, case.rgba);
                assert_eq!(previous.source, case.source);
                assert_eq!(previous.reference_subject, case.reference_subject);
                assert_eq!(previous.candidate_subject, case.candidate_subject);
                assert_eq!(previous.native, case.native);
                assert_eq!(previous.browser, case.browser);
                let stem = root.join(format!("results-{kind}/pair-000-case-{index:03}-candidate"));
                let request_value: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(stem.with_extension("request.json")).unwrap(),
                )
                .unwrap();
                let request: TrialRequest = serde_json::from_value(request_value.clone()).unwrap();
                assert_eq!(serde_json::to_value(&request).unwrap(), request_value);
                validate_case(&request.case).unwrap();
                let result_value: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(stem.with_extension("result.json")).unwrap(),
                )
                .unwrap();
                let result: TrialResult = serde_json::from_value(result_value.clone()).unwrap();
                assert_eq!(serde_json::to_value(&result).unwrap(), result_value);
                assert_eq!(result.output.case, case.identity);
            }
        }
    }

    #[test]
    fn fixed_budget_uses_separate_accepted_roles_and_preserves_s24_fixture() {
        let old_native = experiment(false, true, "fixture".into()).unwrap();
        let old_public = experiment(true, true, "fixture".into()).unwrap();
        let new_native = experiment(false, false, "fixture".into()).unwrap();
        let new_public = experiment(true, false, "fixture".into()).unwrap();
        assert_eq!((old_native.cases.len(), old_public.cases.len()), (5, 5));
        assert_eq!((new_native.cases.len(), new_public.cases.len()), (19, 10));
        let old_workers = (old_native.cases.len() + old_public.cases.len() * 3) * 2 * 2;
        let new_workers = (new_native.cases.len() + new_public.cases.len() * 3) * 2 * 2;
        assert_eq!(
            (old_workers, new_workers, old_workers + new_workers),
            (80, 196, 276)
        );
        let mut policies = std::collections::HashSet::new();
        for plan in [&old_native, &old_public, &new_native, &new_public] {
            assert_eq!(plan.pairs, 2);
            for case in &plan.cases {
                assert_eq!(case.measurement.samples, 20);
                assert_eq!(case.measurement.warmup_ms, 50);
                let ciede_complete = case.name == "cielab-ciede2000-palette64-latency";
                assert_eq!(
                    case.measurement.measurement_ms,
                    if ciede_complete { 10_000 } else { 250 }
                );
                assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                assert_eq!(
                    case.measurement.application_cache,
                    ApplicationCache::NotApplicable
                );
                assert_eq!(
                    case.source,
                    Dimensions {
                        width: 128,
                        height: 96
                    }
                );
                assert_eq!(case.identity.input, old_native.cases[0].identity.input);
                assert_eq!(case.accepted_subject, case.candidate_subject);
                if let Some(NativeOperation::Quantize { settings }) = &case.native {
                    policies.insert(serde_json::to_string(&settings.matching).unwrap());
                    assert_eq!(settings.palette.len(), 64);
                    assert_eq!(settings.palette.last(), Some(&PaletteEntry::Transparent {}));
                    assert_eq!(settings.alpha, AlphaPolicy::Preserve { threshold: 0.5 });
                }
            }
        }
        assert_eq!(policies.len(), 15);
        assert_eq!(
            new_native
                .cases
                .iter()
                .filter(|c| c.measurement.scope == CallScope::NativeForwardConversion)
                .count(),
            2
        );
        assert_eq!(
            new_native
                .cases
                .iter()
                .filter(|c| c.measurement.scope == CallScope::NativeMetricScores)
                .count(),
            7
        );
        for (native, public) in old_native
            .cases
            .iter()
            .zip(&old_public.cases)
            .chain(new_native.cases.iter().zip(&new_public.cases))
        {
            assert_eq!(native.identity, public.identity);
        }
    }
}
