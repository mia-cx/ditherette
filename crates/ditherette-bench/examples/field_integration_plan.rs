//! Declares S26's 208 serial workers. This executable never measures operations.
#[path = "support/fields.rs"]
mod fields;

use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, fields::*, native::*, *,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use ditherette_wasm::bench_subjects::{field_calls, fields::Component};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let source = Dimensions {
        width: 64,
        height: 48,
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
    let mut operations = Vec::new();
    if !public {
        for (name, space) in [
            ("srgb", WorkingSpace::Srgb),
            ("linear-rgb", WorkingSpace::LinearRgb),
            ("oklab", WorkingSpace::Oklab),
            ("oklch", WorkingSpace::Oklch),
            ("cielab", WorkingSpace::Cielab),
            ("cielch", WorkingSpace::Cielch),
            ("ycbcr", WorkingSpace::Ycbcr),
        ] {
            operations.push((
                format!("{name}-inverse-f32-image"),
                NativeOperation::FieldComponent {
                    component: Component::Inverse { space },
                },
                None,
            ));
        }
        for (name, field) in [
            (
                "bayer2",
                Field::Bayer {
                    size: BayerSize::Two,
                },
            ),
            (
                "bayer4",
                Field::Bayer {
                    size: BayerSize::Four,
                },
            ),
            (
                "bayer8",
                Field::Bayer {
                    size: BayerSize::Eight,
                },
            ),
            (
                "bayer16",
                Field::Bayer {
                    size: BayerSize::Sixteen,
                },
            ),
            ("random", Field::Random { seed: fields::SEED }),
        ] {
            operations.push((
                format!("{name}-threshold-grid"),
                NativeOperation::FieldComponent {
                    component: Component::Field { field },
                },
                None,
            ));
        }
        for (name, space, radius) in [
            ("oklab", WorkingSpace::Oklab, 1),
            ("oklch", WorkingSpace::Oklch, 2),
        ] {
            operations.push((
                format!("{name}-adaptive{radius}-mask"),
                NativeOperation::FieldComponent {
                    component: Component::Placement {
                        space,
                        placement: fields::adaptive(radius),
                    },
                },
                None,
            ));
        }
        for (name, space) in [("srgb", WorkingSpace::Srgb), ("oklab", WorkingSpace::Oklab)] {
            operations.push((
                format!("{name}-source-construction-inclusive"),
                NativeOperation::FieldComponent {
                    component: Component::SourceConversion { space },
                },
                None,
            ));
        }
    }
    for (name, operation) in fields::complete_recipes() {
        let native = match &operation {
            PublicOperation::Perturb { settings } => NativeOperation::Perturb {
                settings: *settings,
            },
            PublicOperation::Separable { settings } => NativeOperation::Separable {
                settings: settings.clone(),
            },
            _ => unreachable!("S26 complete recipe"),
        };
        let browser = public.then_some(BrowserCase {
            operation,
            accepted: BrowserBackend::Package,
            candidate: BrowserBackend::Package,
            preparation: BrowserPreparation::PrimedInstance,
            cache: CacheCapability::None,
            measure_nonexact: false,
            progress: None,
        });
        operations.push((name, native, browser));
    }
    let mut cases = Vec::new();
    for (name, native, browser) in operations {
        let subject = match &browser {
            Some(browser) => browser.operation.subject(BrowserBackend::Package),
            None => match &native {
                NativeOperation::FieldComponent { component } => component.prod_subject(),
                NativeOperation::Perturb { .. } => field_calls::PERTURB_SUBJECT,
                NativeOperation::Separable { .. } => field_calls::SEPARABLE_SUBJECT,
                _ => unreachable!("S26 operation"),
            },
        };
        let measurement_ms = if matches!(
            native.scope(),
            CallScope::NativeInverseConversion | CallScope::NativeFieldEvaluation
        ) {
            250
        } else {
            10_000
        };
        cases.push(PairCase {
            name,
            identity: native.identity(source, &rgba)?,
            source,
            rgba: rgba.clone(),
            reference_subject: native.reference_subject().into(),
            // The immutable artifact identifies the revision; both execute the real production call.
            accepted_subject: subject.into(),
            candidate_subject: subject.into(),
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
            "S26 {} baseline versus call-owned converter",
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
        return Err(io::Error::other(
            "usage: field_integration_plan native|public output.json host-load-notes",
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

    #[test]
    fn fixed_208_workers_keep_identical_complete_recipes_and_declared_caps() {
        let native = experiment(false, "fixture".into()).unwrap();
        let public = experiment(true, "fixture".into()).unwrap();
        assert_eq!((native.cases.len(), public.cases.len()), (25, 9));
        assert_eq!(
            (native.cases.len() + public.cases.len() * 3) * native.pairs as usize * 2,
            208
        );
        let mut scopes = std::collections::BTreeMap::new();
        for case in &native.cases {
            *scopes
                .entry(serde_json::to_string(&case.measurement.scope).unwrap())
                .or_insert(0) += 1;
        }
        assert_eq!(scopes.values().sum::<usize>(), 25);
        for (scope, count) in [
            (CallScope::NativeInverseConversion, 7),
            (CallScope::NativeFieldEvaluation, 5),
            (CallScope::NativePlacementMask, 2),
            (CallScope::NativeSourceConversion, 2),
            (CallScope::NativeCompleteCall, 9),
        ] {
            assert_eq!(scopes[&serde_json::to_string(&scope).unwrap()], count);
        }
        for plan in [&native, &public] {
            assert_eq!(plan.pairs, 2);
            let mut identities = std::collections::HashSet::new();
            for case in &plan.cases {
                assert!(identities.insert(case.identity.settings.0));
                assert_eq!(
                    case.source,
                    Dimensions {
                        width: 64,
                        height: 48
                    }
                );
                assert_eq!(case.measurement.samples, 20);
                assert_eq!(case.measurement.warmup_ms, 50);
                assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                assert_eq!(
                    case.measurement.application_cache,
                    ApplicationCache::NotApplicable
                );
                assert_eq!(
                    case.measurement.measurement_ms,
                    if matches!(
                        case.measurement.scope,
                        CallScope::NativeInverseConversion | CallScope::NativeFieldEvaluation
                    ) {
                        250
                    } else {
                        10_000
                    }
                );
                assert_eq!(case.accepted_subject, case.candidate_subject);
            }
        }
        for (native, public) in native.cases[16..].iter().zip(&public.cases) {
            assert_eq!(native.identity, public.identity);
            assert_eq!(native.name, public.name);
            assert_eq!(native.rgba, public.rgba);
            assert!(public.native.is_none());
        }
    }
}
