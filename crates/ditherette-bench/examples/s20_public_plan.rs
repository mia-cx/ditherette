//! Materialize the predeclared public-call matrix without running measurements.

use ditherette_bench::paired::{
    browser::{
        Anchor, BrowserBackend, BrowserCase, BrowserPreparation, CacheCapability, PublicOperation,
    },
    coordinator::validate_experiment,
    ApplicationCache, CallScope, Experiment, Measurement, PairCase, SampleMode,
};
use ditherette_bench_api::verification::{Dimensions, ReferenceState};
use std::{env, fs::OpenOptions, io, io::Write};

/// Reuse the original deterministic nearest fixture and explicit initialization scopes.
pub fn case(
    name: &str,
    source: (u32, u32),
    output: (u32, u32),
    mode: SampleMode,
    preparation: BrowserPreparation,
) -> io::Result<PairCase> {
    let source = Dimensions {
        width: source.0,
        height: source.1,
    };
    let output = Dimensions {
        width: output.0,
        height: output.1,
    };
    let rgba: Vec<_> = (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                [
                    (x * 17 + 11) as u8,
                    (y * 31 + 23) as u8,
                    ((x ^ y) + 47) as u8,
                    (x + y + 127) as u8,
                ]
            })
        })
        .collect();
    let initialization = matches!(
        preparation,
        BrowserPreparation::InitializationBytes | BrowserPreparation::InitializationCompiled
    );
    let operation = PublicOperation::ResizeNearest {
        anchor: Anchor::Center,
    };
    let browser = BrowserCase {
        retained_output_limit_bytes: None,
        execution: None,
        row_policy: None,
        operation,
        accepted: if initialization {
            BrowserBackend::Package
        } else {
            BrowserBackend::TypeScript
        },
        candidate: BrowserBackend::Package,
        preparation,
        cache: CacheCapability::None,
        measure_nonexact: false,
        progress: None,
        threads: None,
    };
    Ok(PairCase {
        native: None,
        name: name.into(),
        identity: browser.operation.identity(source, &rgba, output)?,
        source,
        rgba,
        reference_subject: browser.operation.reference_subject().into(),
        accepted_subject: browser.operation.subject(browser.accepted).into(),
        candidate_subject: browser.operation.subject(browser.candidate).into(),
        measurement: Measurement {
            mode,
            scope: if initialization {
                CallScope::Initialization
            } else {
                CallScope::CompleteCall
            },
            application_cache: ApplicationCache::NotApplicable,
            samples: 100,
            measurement_ms: 1000,
            warmup_ms: 250,
            target_sample_ms: 5,
        },
        browser: Some(browser),
    })
}

fn experiment(host_load_notes: String) -> io::Result<Experiment> {
    use BrowserPreparation::{
        FreshInstance, InitializationBytes, InitializationCompiled, PrimedInstance,
    };
    use SampleMode::{SingleCall, Throughput};
    let cases = [
        (
            "identity-latency",
            (512, 384),
            (512, 384),
            SingleCall,
            PrimedInstance,
        ),
        (
            "identity-throughput",
            (512, 384),
            (512, 384),
            Throughput,
            PrimedInstance,
        ),
        (
            "reduction-latency",
            (1024, 768),
            (512, 384),
            SingleCall,
            PrimedInstance,
        ),
        (
            "reduction-throughput",
            (1024, 768),
            (512, 384),
            Throughput,
            PrimedInstance,
        ),
        (
            "first-reduction-call",
            (1024, 768),
            (512, 384),
            SingleCall,
            FreshInstance,
        ),
        (
            "enlargement-latency",
            (256, 192),
            (1024, 768),
            SingleCall,
            PrimedInstance,
        ),
        (
            "unequal-axes-latency",
            (1024, 256),
            (256, 1024),
            SingleCall,
            PrimedInstance,
        ),
        (
            "initialization-bytes",
            (1, 1),
            (1, 1),
            SingleCall,
            InitializationBytes,
        ),
        (
            "initialization-compiled",
            (1, 1),
            (1, 1),
            SingleCall,
            InitializationCompiled,
        ),
    ]
    .into_iter()
    .map(|(name, source, output, mode, preparation)| case(name, source, output, mode, preparation))
    .collect::<io::Result<Vec<_>>>()?;
    let experiment = Experiment {
        label: "S20 public nearest versus actual TypeScript, with package initialization controls"
            .into(),
        reference_state: ReferenceState::Frozen,
        pairs: 4,
        host_load_notes,
        cases,
    };
    validate_experiment(&experiment)?;
    Ok(experiment)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path, host_load_notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: s20_public_plan NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    let bytes = serde_json::to_vec_pretty(&experiment(host_load_notes.clone())?)
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
    fn matrix_matches_the_declared_budget_and_separates_initialization() {
        let experiment = experiment("controlled fixture, no measurements".into()).unwrap();
        assert_eq!(experiment.pairs * experiment.cases.len() * 2 * 3, 216);
        assert_eq!(
            experiment
                .cases
                .iter()
                .filter(|case| case.measurement.mode == SampleMode::Throughput)
                .count(),
            2
        );
        let initialization: Vec<_> = experiment
            .cases
            .iter()
            .filter(|case| case.measurement.scope == CallScope::Initialization)
            .collect();
        assert_eq!(initialization.len(), 2);
        for case in initialization {
            assert_eq!(
                case.source,
                Dimensions {
                    width: 1,
                    height: 1
                }
            );
            assert_eq!(case.accepted_subject, case.candidate_subject);
        }
        for case in experiment.cases {
            assert_eq!(case.measurement.samples, 100);
            assert_eq!(
                case.rgba.len(),
                case.source.width as usize * case.source.height as usize * 4
            );
        }
    }
}
