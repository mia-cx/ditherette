use std::collections::BTreeMap;
use std::process::Command;

fn output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    for path in [
        "src",
        "build.rs",
        "Cargo.toml",
        "Cargo.lock",
        "../ditherette-bench-api/src",
        "../ditherette-wasm/src",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    for name in ["HEAD", "index"] {
        if let Some(path) = output("git", &["rev-parse", "--git-path", name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(branch) = output("git", &["symbolic-ref", "HEAD"]) {
        if let Some(path) = output("git", &["rev-parse", "--git-path", &branch]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    let revision = output("git", &["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = output("git", &["status", "--porcelain"]).is_none_or(|status| !status.is_empty());
    let compiler = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let rustc = output(&compiler, &["--version", "--verbose"])
        .unwrap_or_else(|| "unknown".into())
        .replace('\n', "; ");
    println!("cargo:rustc-env=DITHERETTE_BENCH_REVISION={revision}");
    println!("cargo:rustc-env=DITHERETTE_BENCH_DIRTY={dirty}");
    println!("cargo:rustc-env=DITHERETTE_BENCH_RUSTC={rustc}");
    let mut configuration = BTreeMap::new();
    for name in [
        "HOST",
        "TARGET",
        "PROFILE",
        "OPT_LEVEL",
        "DEBUG",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_LINKER",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
        configuration.insert(name.to_owned(), std::env::var(name).unwrap_or_default());
    }
    for (name, value) in std::env::vars() {
        if name.starts_with("CARGO_FEATURE_")
            || name.starts_with("CARGO_CFG_")
            || name.starts_with("CARGO_PROFILE_")
        {
            println!("cargo:rerun-if-env-changed={name}");
            configuration.insert(name, value);
        }
    }
    println!("cargo:rustc-env=DITHERETTE_BENCH_CONFIGURATION={configuration:?}");
    println!("cargo:rustc-env=DITHERETTE_BENCH_RECORDED_BUILD=false");
}
