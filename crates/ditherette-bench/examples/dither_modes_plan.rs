//! Write the dither-modes plan for the existing pair coordinator. Never execute measurements.

use ditherette_bench::paired::scalar::{dither_modes_experiment, Comparison};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [mode, path, notes] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: dither_modes_plan spec-prod|prod-prod NEW_JSON HOST_LOAD_NOTES",
        ));
    };
    let comparison = match mode.as_str() {
        "spec-prod" => Comparison::SpecProd,
        "prod-prod" => Comparison::ProdProd,
        _ => return Err(io::Error::other("mode must be spec-prod or prod-prod")),
    };
    let plan = dither_modes_experiment(comparison, notes.clone())?;
    let bytes = serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&bytes)
}
