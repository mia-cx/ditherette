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
mod paired_browser;
mod paired_native;
mod registry;
mod report;
mod result;
mod runtime;
mod tiling_sweep;
mod util;
mod wasm_resize;

use ditherette_bench::lease::{require_quiet, BenchmarkGuard};
use std::{env, process::ExitCode};

use cli::help_text;
use commands::{comp_command, describe_subject, list_subjects, perf_command, tile_command};
use error::BenchError;
use manifest::expand_command;
use registry::Registry;
use tiling_sweep::tiling_sweep_command;
use wasm_resize::wasm_resize_command;

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
    let guard = BenchmarkGuard::acquire().map_err(BenchError::io)?;
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("{}", help_text());
        return Ok(());
    }

    let command = args.remove(0);
    let expanded = expand_command(command, args)?;
    if matches!(
        expanded.command.as_str(),
        "perf"
            | "comp"
            | "tile"
            | "tiling-sweep"
            | "wasm-resize"
            | "paired-trial"
            | "paired-browser-trial"
    ) {
        require_quiet().map_err(BenchError::io)?;
    }
    let registry = Registry::load();

    match expanded.command.as_str() {
        "paired-trial" => paired_native::run(&registry, &expanded.args),
        "paired-browser-trial" => paired_browser::run(&guard.lease, &registry, &expanded.args),
        "list-subjects" => list_subjects(&registry, &expanded.args),
        "describe-subject" => describe_subject(&registry, &expanded.args),
        "perf" => perf_command(&registry, &expanded.args),
        "comp" => comp_command(&registry, &expanded.args),
        "tile" => tile_command(&registry, &expanded.args),
        "tiling-sweep" => tiling_sweep_command(&registry, &expanded.args),
        "wasm-resize" => wasm_resize_command(&guard.lease, &expanded.args),
        unknown => Err(BenchError::Config(format!("unknown command {unknown:?}"))),
    }
}
