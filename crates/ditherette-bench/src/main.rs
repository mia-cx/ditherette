//! Custom image-processing benchmark harness for Ditherette.
//!
//! This binary owns benchmark orchestration while implementation crates expose
//! typed benchmark subjects through `ditherette-bench-api`.

mod baseline;
mod case;
mod cli;
mod commands;
mod compare;
mod error;
mod fixture;
mod manifest;
mod measure;
mod registry;
mod report;
mod result;
mod runtime;
mod tiling_sweep;
mod util;

use std::{env, process::ExitCode};

use cli::help_text;
use commands::{comp_command, describe_subject, list_subjects, perf_command, tile_command};
use error::BenchError;
use manifest::expand_command;
use registry::Registry;
use tiling_sweep::tiling_sweep_command;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            error.exit_code()
        }
    }
}

fn run() -> Result<(), BenchError> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("{}", help_text());
        return Ok(());
    }

    let command = args.remove(0);
    let expanded = expand_command(command, args)?;
    let registry = Registry::load();

    match expanded.command.as_str() {
        "list-subjects" => list_subjects(&registry, &expanded.args),
        "describe-subject" => describe_subject(&registry, &expanded.args),
        "perf" => perf_command(&registry, &expanded.args),
        "comp" => comp_command(&registry, &expanded.args),
        "tile" => tile_command(&registry, &expanded.args),
        "tiling-sweep" => tiling_sweep_command(&registry, &expanded.args),
        unknown => Err(BenchError::Config(format!("unknown command {unknown:?}"))),
    }
}
