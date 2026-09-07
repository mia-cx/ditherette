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
        _ => Err(io::Error::other("usage: ditherette-bench-pair prepare EXPERIMENT ACCEPTED FULL_REV CANDIDATE FULL_REV NEW_DIRECTORY | run PREPARED_JSON NEW_RESULTS_DIRECTORY (requires DITHERETTE_BENCH_QUIET=1)")),
    }
}
