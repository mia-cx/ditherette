//! External coordinator. Only its read-only build-info command owns an execution slot itself.

use ditherette_bench::{
    browser_assets::{self, BrowserSources},
    paired::{coordinator, Experiment, Gate, PreparedPair},
};
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
        [command] if command == "build-info" => {
            let _guard = ditherette_bench::lease::BenchmarkGuard::acquire()?;
            println!("{}", ditherette_bench::paired::build_info_json()?);
            Ok(true)
        }
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
            native: None,
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
        [command, install, libraries, directory] if command == "prepare-webkit" => {
            println!("{}", browser_assets::prepare_webkit(Path::new(install), Path::new(libraries), Path::new(directory))?.display());
            Ok(true)
        }
        [command, source, revision] if command == "check-browser-source" => {
            let source = serde_json::from_slice(&fs::read(source)?).map_err(io::Error::other)?;
            browser_assets::validate_source_revision(&source, revision)?;
            Ok(true)
        }
        [command, experiment, accepted, accepted_revision, candidate, candidate_revision, sources, directory] if command == "prepare-browser" => {
            let experiment: Experiment = serde_json::from_slice(&fs::read(experiment)?).map_err(io::Error::other)?;
            let sources: BrowserSources = serde_json::from_slice(&fs::read(sources)?).map_err(io::Error::other)?;
            browser_assets::validate_source_revision(&sources.accepted, accepted_revision)?;
            browser_assets::validate_source_revision(&sources.candidate, candidate_revision)?;
            let directory = Path::new(directory);
            fs::create_dir(directory)?;
            let browser = browser_assets::prepare_assets(&sources, &directory.join("assets"))?;
            coordinator::prepare_with_browser(experiment, (Path::new(accepted), accepted_revision), (Path::new(candidate), candidate_revision), &directory.join("pair"), browser)?;
            println!("{}", directory.join("pair/prepared.json").display());
            Ok(true)
        }
        [command, prepared, directory] if command == "run" => {
            let prepared: PreparedPair = serde_json::from_slice(&fs::read(prepared)?).map_err(io::Error::other)?;
            let report = coordinator::run_with_browser(&prepared, Path::new(directory), browser_assets::validate_trial_assets)?;
            println!("{}", serde_json::to_string_pretty(&report).map_err(io::Error::other)?);
            Ok(report.gate == Gate::Pass)
        }
        _ => Err(io::Error::other("usage: ditherette-bench-pair build-info (guarded read-only provenance) | control-plan NEW_JSON HOST_NOTES | prepare EXPERIMENT ACCEPTED FULL_REV CANDIDATE FULL_REV NEW_DIRECTORY | prepare-webkit INSTALL PRIVATE_LIBRARIES NEW_DIRECTORY | prepare-browser EXPERIMENT ACCEPTED FULL_REV CANDIDATE FULL_REV SOURCES_JSON NEW_DIRECTORY | run PREPARED_JSON NEW_RESULTS_DIRECTORY (requires DITHERETTE_BENCH_QUIET=1)")),
    }
}
