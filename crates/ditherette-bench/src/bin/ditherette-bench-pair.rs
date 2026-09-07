//! External coordinator. It never owns a benchmark execution slot itself.

use ditherette_bench::paired::{coordinator, Experiment, Gate, PreparedPair};
use std::{env, fs, io, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("paired benchmark: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<bool> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, path, notes] if command == "control-plan" => {
            use ditherette_bench::{paired::*, verification::{input_digest, settings_digest}};
            use ditherette_bench_api::verification::*;
            use std::io::Write;
            let source = Dimensions { width: 512, height: 384 };
            let output = Dimensions { width: 256, height: 192 };
            let rgba: Vec<_> = (0..source.height).flat_map(|y| (0..source.width).flat_map(move |x| [x as u8, y as u8, (x ^ y) as u8, 255])).collect();
            let semantics = SemanticIdentity { operation: Operation::Resize, recipe: "nearest-center-default".into(), version: 1, space: None };
            let identity = CaseIdentity { settings: settings_digest(&(semantics.clone(), output, "center-default")).map_err(io::Error::other)?,
                semantics, input: input_digest(source, &rgba), output };
            let cases = [SampleMode::SingleCall, SampleMode::Throughput].into_iter().map(|mode| PairCase {
                browser: None,
                name: match mode { SampleMode::SingleCall => "nearest-latency", SampleMode::Throughput => "nearest-throughput" }.into(),
                identity: identity.clone(), source, rgba: rgba.clone(), reference_subject: "spec:resize:nearest:scalar".into(),
                accepted_subject: "spec:resize:nearest:scalar".into(), candidate_subject: "spec:resize:nearest:scalar".into(),
                measurement: Measurement { mode, scope: CallScope::NativeKernel, application_cache: ApplicationCache::NotApplicable,
                    samples: 100, measurement_ms: 1000, warmup_ms: 250, target_sample_ms: 5 } }).collect();
            let experiment = Experiment { label: "S06 cross-revision native reference controls, not accepted production".into(),
                reference_state: ReferenceState::PreFreeze, pairs: 4, host_load_notes: notes.clone(), cases };
            let bytes = serde_json::to_vec_pretty(&experiment).map_err(io::Error::other)?;
            fs::OpenOptions::new().create_new(true).write(true).open(path)?.write_all(&bytes)?;
            Ok(true)
        }
        [command, experiment, accepted, accepted_revision, candidate, candidate_revision, directory] if command == "prepare" => {
            let experiment: Experiment = serde_json::from_slice(&fs::read(experiment)?).map_err(io::Error::other)?;
            coordinator::prepare(experiment, (Path::new(accepted), accepted_revision), (Path::new(candidate), candidate_revision), Path::new(directory))?;
            Ok(true)
        }
        [command, prepared, directory] if command == "run" => {
            let prepared: PreparedPair = serde_json::from_slice(&fs::read(prepared)?).map_err(io::Error::other)?;
            let report = coordinator::run(&prepared, Path::new(directory))?;
            println!("{}", serde_json::to_string_pretty(&report).map_err(io::Error::other)?);
            Ok(report.gate == Gate::Pass)
        }
        _ => Err(io::Error::other("usage: ditherette-bench-pair control-plan NEW_JSON HOST_NOTES | prepare EXPERIMENT ACCEPTED FULL_REV CANDIDATE FULL_REV NEW_DIRECTORY | run PREPARED_JSON NEW_RESULTS_DIRECTORY (requires DITHERETTE_BENCH_QUIET=1)")),
    }
}
