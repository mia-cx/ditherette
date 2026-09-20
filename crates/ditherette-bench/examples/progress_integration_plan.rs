//! S33's two cold public comparisons. Declaration only; no measurements run here.
#[allow(dead_code)]
#[path = "stage_integration_plan.rs"]
mod stage_plan;

use ditherette_bench::paired::{
    browser::{
        BrowserBackend, PreparationCapability, ProgressMode, ProgressRoles, PublicOperation,
    },
    coordinator::validate_experiment,
    *,
};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

pub fn experiment(callbacks: bool, notes: String) -> io::Result<Experiment> {
    let mut plan = stage_plan::experiment(true, notes)?;
    plan.label = if callbacks {
        "progress-callback-overhead"
    } else {
        "progress-disabled-regression"
    }
    .into();
    plan.cases
        .retain(|case| case.measurement.application_cache == ApplicationCache::Cold);
    let mut perturb = plan
        .cases
        .iter()
        .find(|case| {
            matches!(
                case.browser.as_ref().unwrap().operation,
                PublicOperation::Separable { .. }
            )
        })
        .expect("declared separable fixture")
        .clone();
    let browser = perturb.browser.as_mut().unwrap();
    let PublicOperation::Separable { settings } = &browser.operation else {
        unreachable!()
    };
    browser.operation = PublicOperation::Perturb {
        settings: settings.perturb,
    };
    perturb.name = "perturb-bayer4-oklab-65x49-cold".into();
    perturb.identity = browser
        .operation
        .identity(perturb.source, &perturb.rgba, perturb.source)?;
    perturb.reference_subject = browser.operation.reference_subject().into();
    perturb.accepted_subject = browser.operation.subject(BrowserBackend::Package).into();
    perturb.candidate_subject = perturb.accepted_subject.clone();
    plan.cases.push(perturb);
    for case in &mut plan.cases {
        let browser = case.browser.as_mut().unwrap();
        browser.cache = browser::CacheCapability::Roles {
            accepted: PreparationCapability::ImageStages,
            candidate: PreparationCapability::ImageStages,
            sample_prime: None,
        };
        browser.progress = Some(ProgressRoles {
            accepted: ProgressMode::Disabled,
            candidate: if callbacks {
                ProgressMode::Enabled
            } else {
                ProgressMode::Disabled
            },
        });
    }
    validate_experiment(&plan)?;
    Ok(plan)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [comparison, destination, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: progress_integration_plan regression|callbacks DESTINATION HOST_LOAD_NOTES",
        ));
    };
    if !matches!(comparison.as_str(), "regression" | "callbacks") {
        return Err(io::Error::other("unknown progress comparison"));
    }
    let plan = experiment(comparison == "callbacks", notes.clone())?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ditherette_wasm::bench_subjects::{field_calls, preparation::CompleteCall};

    #[test]
    fn two_comparisons_reuse_five_cold_workloads_without_changing_semantic_identity() {
        let disabled = experiment(false, "untimed fixture".into()).unwrap();
        let enabled = experiment(true, "untimed fixture".into()).unwrap();
        assert_eq!(disabled.cases.len(), 5);
        assert_eq!(disabled.cases.len() * disabled.pairs * 2 * 3 * 2, 120);
        let historical = stage_plan::experiment(true, "untimed fixture".into()).unwrap();
        for (off, on) in disabled.cases.iter().zip(&enabled.cases) {
            assert_eq!(off.identity, on.identity);
            assert_eq!(off.rgba, on.rgba);
            assert_eq!(off.measurement, on.measurement);
            assert_eq!(off.measurement.samples, 20);
            assert_eq!(off.measurement.warmup_ms, 50);
            assert_eq!(off.measurement.measurement_ms, 10_000);
            let browser = on.browser.as_ref().unwrap();
            assert_eq!(
                browser.preparation,
                browser::BrowserPreparation::FreshInstance
            );
            assert_eq!(browser.progress.unwrap().candidate, ProgressMode::Enabled);
            assert_eq!(
                off.browser.as_ref().unwrap().progress.unwrap().candidate,
                ProgressMode::Disabled
            );
            if let Some(old) = historical.cases.iter().find(|old| old.name == off.name) {
                assert_eq!(old.identity, off.identity);
                assert_eq!(old.rgba, off.rgba);
            } else {
                let old = historical
                    .cases
                    .iter()
                    .find(|old| {
                        matches!(
                            old.browser.as_ref().unwrap().operation,
                            PublicOperation::Separable { .. }
                        )
                    })
                    .unwrap();
                assert_eq!(old.source, off.source);
                assert_eq!(old.rgba, off.rgba);
                let PublicOperation::Separable { settings } =
                    &old.browser.as_ref().unwrap().operation
                else {
                    unreachable!()
                };
                assert_eq!(
                    browser.operation,
                    PublicOperation::Perturb {
                        settings: settings.perturb
                    }
                );
            }
        }
    }

    #[test]
    fn all_five_measured_fixtures_match_the_target_local_frozen_oracle_without_timing() {
        let native = stage_plan::experiment(false, "untimed frozen check".into()).unwrap();
        for case in experiment(true, "untimed frozen check".into())
            .unwrap()
            .cases
        {
            let browser = case.browser.as_ref().unwrap();
            let reference: ditherette_bench_oracle::OracleRequest =
                serde_json::from_value(serde_json::json!({
                    "source":case.source, "rgba":case.rgba, "output":case.identity.output,
                    "operation":browser.operation, "identity":case.identity
                }))
                .unwrap();
            let expected = reference.execute().unwrap().output;
            let request = if let Some(request) = browser
                .operation
                .processing_request(case.source, &case.rgba)
                .unwrap()
            {
                request
            } else {
                let old = native
                    .cases
                    .iter()
                    .find(|old| old.name == case.name)
                    .unwrap();
                let native::NativeOperation::Processor { settings, .. } =
                    old.native.as_ref().unwrap()
                else {
                    unreachable!()
                };
                settings.reference_request(case.source, &case.rgba).unwrap()
            };
            let mut processor = field_calls::processor().unwrap();
            let actual = CompleteCall::new(&request)
                .unwrap()
                .output(&mut processor, &case.rgba)
                .unwrap()
                .verification();
            assert_eq!(actual, expected, "{}", case.name);
        }
    }
}
