//! Declare S34 startup scopes without executing initialization or timing measurements.
#[allow(dead_code)]
#[path = "s20_public_plan.rs"]
mod public_plan;

use ditherette_bench::paired::{
    browser::{BrowserExecution, BrowserPreparation, ThreadRoles, Threads},
    coordinator::validate_experiment,
    Experiment, SampleMode,
};
use ditherette_bench_api::verification::ReferenceState;
use std::{env, fs::OpenOptions, io, io::Write};

fn experiment(threaded: bool, host_load_notes: String) -> io::Result<Experiment> {
    let policy = if threaded {
        Threads::Required
    } else {
        Threads::Disabled
    };
    let cases = [
        (
            "initialization-bytes",
            BrowserPreparation::InitializationBytes,
        ),
        (
            "initialization-compiled",
            BrowserPreparation::InitializationCompiled,
        ),
    ]
    .into_iter()
    .map(|(name, preparation)| {
        let mut case =
            public_plan::case(name, (1, 1), (1, 1), SampleMode::SingleCall, preparation)?;
        case.browser.as_mut().unwrap().execution = threaded.then_some(BrowserExecution::HostWorker);
        case.browser.as_mut().unwrap().threads = Some(ThreadRoles {
            accepted: policy,
            candidate: policy,
        });
        case.measurement.samples = 20;
        case.measurement.warmup_ms = 50;
        case.measurement.measurement_ms = 10_000;
        Ok(case)
    })
    .collect::<io::Result<Vec<_>>>()?;
    let plan = Experiment {
        label: if threaded {
            "startup-required-control"
        } else {
            "startup-scalar-regression"
        }
        .into(),
        reference_state: ReferenceState::Frozen,
        pairs: 2,
        host_load_notes,
        cases,
    };
    validate_experiment(&plan)?;
    Ok(plan)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [comparison, destination, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: startup_integration_plan regression|threaded NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    if !matches!(comparison.as_str(), "regression" | "threaded") {
        return Err(io::Error::other("unknown startup comparison"));
    }
    let plan = experiment(comparison == "threaded", notes.clone())?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?
        .write_all(&serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ditherette_bench::paired::{ApplicationCache, CallScope};
    use ditherette_bench_api::verification::{Dimensions, Pixels};

    #[test]
    fn fixed_startup_matrix_reuses_both_scopes_and_the_exact_frozen_probe() {
        let scalar = experiment(false, "untimed fixture".into()).unwrap();
        let threaded = experiment(true, "untimed fixture".into()).unwrap();
        assert_eq!(scalar.cases.len(), 2);
        assert_eq!(scalar.cases.len() * scalar.pairs * 2 * 3 * 2, 48);
        for (off, on) in scalar.cases.iter().zip(&threaded.cases) {
            assert_eq!(off.identity, on.identity);
            assert_eq!(off.rgba, [11, 23, 47, 127]);
            assert_eq!(off.rgba, on.rgba);
            assert_eq!(off.measurement, on.measurement);
            assert_eq!(
                off.source,
                Dimensions {
                    width: 1,
                    height: 1
                }
            );
            assert_eq!(off.identity.output, off.source);
            assert_eq!(off.measurement.scope, CallScope::Initialization);
            assert_eq!(off.measurement.mode, SampleMode::SingleCall);
            assert_eq!(
                off.measurement.application_cache,
                ApplicationCache::NotApplicable
            );
            assert_eq!(off.measurement.samples, 20);
            assert_eq!(off.measurement.warmup_ms, 50);
            assert_eq!(off.measurement.measurement_ms, 10_000);
            for (case, policy) in [(off, Threads::Disabled), (on, Threads::Required)] {
                let browser = case.browser.as_ref().unwrap();
                assert_eq!(
                    browser.execution,
                    (policy == Threads::Required).then_some(BrowserExecution::HostWorker)
                );
                assert_eq!(
                    browser.threads,
                    Some(ThreadRoles {
                        accepted: policy,
                        candidate: policy
                    })
                );
                let request: ditherette_bench_oracle::OracleRequest =
                    serde_json::from_value(serde_json::json!({
                        "source":case.source, "rgba":case.rgba, "output":case.identity.output,
                        "operation":browser.operation, "identity":case.identity
                    }))
                    .unwrap();
                let expected = request.execute().unwrap().output;
                assert_eq!(
                    expected.pixels,
                    Pixels::Rgba8 {
                        data: case.rgba.clone()
                    }
                );
                assert!(expected.warnings.is_empty());
            }
        }
    }
}
