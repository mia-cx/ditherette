//! Write one scalar plan for the existing pair coordinator. Never execute measurements.

use ditherette_bench::paired::scalar::{experiment, Comparison};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [mode, path, notes, prefixes @ ..] = args.as_slice() else {
        return Err(io::Error::other("usage: scalar_spec_plan spec-prod|prod-prod NEW_JSON HOST_LOAD_NOTES [CASE_PREFIX ...]"));
    };
    let comparison = match mode.as_str() {
        "spec-prod" => Comparison::SpecProd,
        "prod-prod" => Comparison::ProdProd,
        _ => return Err(io::Error::other("mode must be spec-prod or prod-prod")),
    };
    let mut plan = experiment(comparison, notes.clone())?;
    if !prefixes.is_empty() {
        for prefix in prefixes {
            if !plan.cases.iter().any(|case| case.name.starts_with(prefix)) {
                return Err(io::Error::other(format!("no case matches {prefix:?}")));
            }
        }
        plan.cases
            .retain(|case| prefixes.iter().any(|prefix| case.name.starts_with(prefix)));
    }
    let bytes = serde_json::to_vec_pretty(&plan).map_err(io::Error::other)?;
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&bytes)
}
