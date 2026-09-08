//! Prepare bounded resize comparisons. This program never runs measurements.

use ditherette_bench::paired::{
    browser::{
        Anchor, BrowserBackend, BrowserCase, BrowserPreparation, CacheCapability, PublicOperation,
        Support,
    },
    coordinator::validate_experiment,
    ApplicationCache, CallScope, Experiment, Measurement, PairCase, SampleMode,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use std::{env, fs::OpenOptions, io, io::Write};

fn operations(slice: &str) -> io::Result<Vec<PublicOperation>> {
    let anchor = Anchor::Center;
    match slice {
        "s21" => Ok(vec![
            PublicOperation::ResizeArea {},
            PublicOperation::ResizeBilinear { anchor },
        ]),
        "s22" => Ok([Support::Fixed, Support::ScaleAware]
            .into_iter()
            .flat_map(|support| {
                [
                    PublicOperation::ResizeBicubic { anchor, support },
                    PublicOperation::ResizeLanczos2 { anchor, support },
                    PublicOperation::ResizeLanczos3 { anchor, support },
                ]
            })
            .collect()),
        "s23" => Ok(vec![PublicOperation::ResizeTrilinear { anchor }]),
        _ => Err(io::Error::other("slice must be s21, s22, or s23")),
    }
}

fn fixture(source: Dimensions) -> Vec<u8> {
    (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                // Opaque input makes the website's premultiplication irrelevant.
                // Transparency remains in untimed frozen/landed conformance tests.
                [
                    (x * 17 + y * 7 + 11) as u8,
                    (y * 31 + x * 3 + 23) as u8,
                    ((x ^ y) + 47) as u8,
                    255,
                ]
            })
        })
        .collect()
}

fn experiment(slice: &str, kind: &str, host_load_notes: String) -> io::Result<Experiment> {
    let public = match kind {
        "public" => true,
        "native" => false,
        _ => return Err(io::Error::other("kind must be native or public")),
    };
    let mut cases = Vec::new();
    for operation in operations(slice)? {
        let reference = operation.reference_subject();
        let mut parts = reference.split(':');
        let filter = parts.nth(2).expect("registered filter");
        let variant = parts.next().expect("registered variant");
        let budgeted_variant = match operation {
            PublicOperation::ResizeArea {} | PublicOperation::ResizeBilinear { .. } => "budgeted",
            PublicOperation::ResizeTrilinear { .. } => "mip-area",
            PublicOperation::ResizeBicubic { support, .. }
            | PublicOperation::ResizeLanczos2 { support, .. }
            | PublicOperation::ResizeLanczos3 { support, .. } => match support {
                Support::Fixed => "budgeted-fixed",
                Support::ScaleAware => "budgeted-scale-aware",
            },
            _ => unreachable!("matrix includes S21/S22/S23 only"),
        };
        let mut shapes = vec![
            (
                "reduction-latency",
                (512, 384),
                (173, 129),
                SampleMode::SingleCall,
            ),
            (
                "reduction-throughput",
                (512, 384),
                (173, 129),
                SampleMode::Throughput,
            ),
        ];
        if slice == "s23" {
            shapes = vec![
                (
                    "fractional-shallow",
                    (512, 384),
                    (173, 129),
                    SampleMode::SingleCall,
                ),
                (
                    "fractional-deep-latency",
                    (512, 384),
                    (53, 41),
                    SampleMode::SingleCall,
                ),
                (
                    "fractional-deep-throughput",
                    (512, 384),
                    (53, 41),
                    SampleMode::Throughput,
                ),
                ("integer-lod", (512, 384), (64, 48), SampleMode::SingleCall),
                ("enlargement", (128, 96), (389, 291), SampleMode::SingleCall),
            ];
        }
        if public && slice == "s21" {
            shapes.extend([
                (
                    "first-enlargement",
                    (128, 96),
                    (389, 291),
                    SampleMode::SingleCall,
                ),
                (
                    "unequal-axes",
                    (512, 96),
                    (129, 383),
                    SampleMode::SingleCall,
                ),
            ]);
        }
        for (shape, (sw, sh), (ow, oh), mode) in shapes {
            let source = Dimensions {
                width: sw,
                height: sh,
            };
            let output = Dimensions {
                width: ow,
                height: oh,
            };
            let rgba = fixture(source);
            let browser = public.then(|| BrowserCase {
                operation: operation.clone(),
                // The website has no bicubic implementation. This is explicitly
                // a package/package sequencing control, not a speedup claim.
                accepted: if matches!(
                    operation,
                    PublicOperation::ResizeBicubic { .. } | PublicOperation::ResizeTrilinear { .. }
                ) {
                    BrowserBackend::Package
                } else {
                    BrowserBackend::TypeScript
                },
                candidate: BrowserBackend::Package,
                preparation: if shape == "first-enlargement" {
                    BrowserPreparation::FreshInstance
                } else {
                    BrowserPreparation::PrimedInstance
                },
                cache: CacheCapability::None,
                progress: None,
                measure_nonexact: slice != "s23",
            });
            let (identity, accepted_subject, candidate_subject) = if let Some(browser) = &browser {
                (
                    operation.identity(source, &rgba, output)?,
                    operation.subject(browser.accepted).into(),
                    operation.subject(browser.candidate).into(),
                )
            } else {
                (
                    ditherette_bench::paired::native::identity(reference, source, &rgba, output)?,
                    format!("prod:resize:{filter}:{variant}"),
                    if slice == "s23" {
                        format!("prod:resize:{filter}:{variant}")
                    } else {
                        format!("candidate:resize:{filter}:{budgeted_variant}")
                    },
                )
            };
            cases.push(PairCase {
                native: None,
                name: format!("{filter}-{variant}-{shape}"),
                identity,
                source,
                rgba,
                reference_subject: reference.into(),
                accepted_subject,
                candidate_subject,
                measurement: Measurement {
                    mode,
                    scope: if public {
                        CallScope::CompleteCall
                    } else {
                        CallScope::NativeKernel
                    },
                    application_cache: ApplicationCache::NotApplicable,
                    samples: 20,
                    measurement_ms: 250,
                    warmup_ms: 50,
                    target_sample_ms: 2,
                },
                browser,
            });
        }
    }
    let experiment = Experiment {
        label: format!("{slice} {kind} landed resize integration diagnostics"),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes,
        cases,
    };
    validate_experiment(&experiment)?;
    Ok(experiment)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [slice, kind, path, host_load_notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: resize_integration_plan s21|s22|s23 native|public NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    let bytes = serde_json::to_vec_pretty(&experiment(slice, kind, host_load_notes.clone())?)
        .map_err(io::Error::other)?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trilinear_budget_has_80_exact_serial_workers() {
        let mut workers = 0;
        for kind in ["native", "public"] {
            let plan = experiment("s23", kind, "fixture only".into()).unwrap();
            assert_eq!(plan.cases.len(), 5);
            workers += plan.pairs * 2 * plan.cases.len() * if kind == "public" { 3 } else { 1 };
            for case in plan.cases {
                assert_eq!(case.reference_subject, "spec:resize:trilinear:mip-area");
                assert_eq!(case.measurement.samples, 20);
                if let Some(browser) = case.browser {
                    assert!(!browser.measure_nonexact);
                    assert_eq!(browser.accepted, BrowserBackend::Package);
                    assert_eq!(browser.candidate, BrowserBackend::Package);
                }
            }
        }
        assert_eq!(workers, 80);
    }

    #[test]
    fn bounded_matrix_has_304_serial_workers_across_native_and_three_engines() {
        let mut workers = 0;
        for (slice, kind, expected_cases) in [
            ("s21", "native", 4),
            ("s21", "public", 8),
            ("s22", "native", 12),
            ("s22", "public", 12),
        ] {
            let plan = experiment(slice, kind, "fixture only; no measurements".into()).unwrap();
            assert_eq!(plan.cases.len(), expected_cases);
            workers += plan.pairs * 2 * plan.cases.len() * if kind == "public" { 3 } else { 1 };
            for case in plan.cases {
                assert!(case.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
                assert_eq!(case.measurement.samples, 20);
                if let Some(browser) = case.browser {
                    assert!(browser.measure_nonexact);
                    assert_eq!(
                        browser.accepted == BrowserBackend::Package,
                        matches!(browser.operation, PublicOperation::ResizeBicubic { .. })
                    );
                }
            }
        }
        assert_eq!(workers, 304);
    }
}
