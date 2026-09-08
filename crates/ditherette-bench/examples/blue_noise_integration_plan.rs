//! Declare 68 self-paired S27 baseline workers. No operation is measured here.
#[path = "support/blue_noise.rs"]
mod blue_noise;

use ditherette_bench::paired::{
    browser::*, coordinator::validate_experiment, fields::Field, native::NativeOperation, *,
};
use ditherette_bench_api::verification::ReferenceState;
use ditherette_wasm::bench_subjects::{field_calls, fields::Component};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn experiment(public: bool, notes: String) -> io::Result<Experiment> {
    let (source, rgba) = blue_noise::fixture();
    let mut recipes = blue_noise::recipes();
    if !public {
        recipes.insert(
            0,
            (
                "blue-noise-threshold-grid".into(),
                NativeOperation::FieldComponent {
                    component: Component::Field {
                        field: Field::BlueNoise {},
                    },
                },
            ),
        );
    }
    let mut cases = Vec::new();
    for (name, native) in recipes {
        let browser = public.then(|| BrowserCase {
            operation: blue_noise::public(&native),
            accepted: BrowserBackend::Package,
            candidate: BrowserBackend::Package,
            preparation: BrowserPreparation::PrimedInstance,
            cache: CacheCapability::None,
            measure_nonexact: false,
        });
        let subject = match &browser {
            Some(browser) => browser.operation.subject(BrowserBackend::Package),
            None => match &native {
                NativeOperation::FieldComponent { component } => component.prod_subject(),
                NativeOperation::Perturb { .. } => field_calls::PERTURB_SUBJECT,
                NativeOperation::Separable { .. } => field_calls::SEPARABLE_SUBJECT,
                _ => unreachable!("S27 operation"),
            },
        };
        let threshold = native.scope() == CallScope::NativeFieldEvaluation;
        cases.push(PairCase {
            name,
            identity: native.identity(source, &rgba)?,
            source,
            rgba: rgba.clone(),
            reference_subject: native.reference_subject().into(),
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
                measurement_ms: if threshold { 250 } else { 10_000 },
                target_sample_ms: 2,
            },
            browser,
            native: (!public).then_some(native),
        });
    }
    let experiment = Experiment {
        label: format!(
            "S27 {} exact blue-noise self-paired baseline",
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
            "usage: blue_noise_integration_plan native|public NEW_JSON HOST_LOAD_NOTES",
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
    fn fixed_68_workers_bind_identical_complete_recipes_and_tile_crossings() {
        let native = experiment(false, "fixture".into()).unwrap();
        let public = experiment(true, "fixture".into()).unwrap();
        assert_eq!((native.cases.len(), public.cases.len()), (5, 4));
        assert_eq!(
            (native.cases.len() + public.cases.len() * 3) * native.pairs * 2,
            68
        );
        assert_eq!(
            native.cases[0].measurement.scope,
            CallScope::NativeFieldEvaluation
        );
        assert_eq!(native.cases[0].measurement.measurement_ms, 250);
        assert_eq!(native.cases[0].identity.semantics.space, None);
        for (native, public) in native.cases[1..].iter().zip(&public.cases) {
            assert_eq!(native.identity, public.identity);
            assert_eq!(native.measurement.scope, CallScope::NativeCompleteCall);
            assert_eq!(public.measurement.scope, CallScope::CompleteCall);
            assert_eq!(native.measurement.measurement_ms, 10_000);
        }
        for plan in [&native, &public] {
            assert_eq!(plan.pairs, 2);
            for case in &plan.cases {
                assert_eq!((case.source.width, case.source.height), (65, 33));
                assert_eq!(
                    (case.measurement.samples, case.measurement.warmup_ms),
                    (20, 50)
                );
                assert_eq!(case.measurement.mode, SampleMode::SingleCall);
                assert_eq!(case.accepted_subject, case.candidate_subject);
            }
        }
    }
}
