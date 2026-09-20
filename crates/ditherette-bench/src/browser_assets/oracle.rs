//! Frozen oracle evidence lives inside the existing complete scripts asset closure.

use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    frozen: Frozen,
    target: String,
    profile: Profile,
    tools: Vec<BuildTool>,
    standard_library: Vec<BuildFile>,
    dependencies: Vec<Dependency>,
    inputs: Vec<BuildFile>,
    files: Vec<BuildFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frozen {
    state: String,
    revision: String,
    artifact: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    features: Vec<String>,
    release: bool,
    opt_level: String,
    wasm_opt: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency {
    name: String,
    version: String,
    source: String,
    checksum: String,
    features: Vec<String>,
}

/// Missing, substituted, or incomplete oracle evidence fails before a browser can launch.
pub fn validate_oracle(bundle: &AssetBundle) -> io::Result<()> {
    let entry = "scripts/oracle/manifest.json";
    require_entry(&bundle.tree, entry)?;
    require_entry(&bundle.tree, "scripts/benchmark-oracle-page.mjs")?;
    let manifest: Manifest = serde_json::from_slice(&fs::read(bundle.tree.root.join(entry))?)
        .map_err(io::Error::other)?;
    let provenance: BuildProvenance = serde_json::from_slice(&fs::read(
        bundle.tree.root.join("provenance/build-provenance.json"),
    )?)
    .map_err(io::Error::other)?;
    let expected: Vec<_> = provenance
        .inputs
        .iter()
        .filter(|file| oracle_source(&file.path))
        .cloned()
        .collect();
    if manifest.inputs.is_empty() || manifest.inputs != expected {
        return Err(invalid(
            "Wasm oracle source inputs differ from complete build provenance",
        ));
    }
    if manifest.schema != 1
        || manifest.target != "wasm32-unknown-unknown"
        || manifest.frozen.state != "frozen"
        || manifest.frozen.revision != "cef2b60a635fd43c3b8e7cb880b5c92fe77d640b"
        || manifest.frozen.artifact
            != "sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe"
        || !manifest.profile.release
        || manifest.profile.opt_level != "s"
        || manifest.profile.wasm_opt
        || manifest.profile.features != ["frozen-build"]
        || manifest.standard_library.is_empty()
        || manifest.dependencies.is_empty()
    {
        return Err(invalid("Wasm oracle frozen/build identity differs"));
    }
    for name in ["rustc", "cargo", "wasm-bindgen"] {
        if !manifest
            .tools
            .iter()
            .any(|tool| tool.name == name && !tool.version.is_empty() && tool.digest.0 != [0; 32])
        {
            return Err(invalid("Wasm oracle toolchain identity is incomplete"));
        }
    }
    for dependency in &manifest.dependencies {
        if dependency.name.is_empty()
            || dependency.version.is_empty()
            || dependency.source != "registry+https://github.com/rust-lang/crates.io-index"
            || dependency.checksum.len() != 64
            || dependency
                .features
                .iter()
                .any(|feature| feature == "preserve_order")
        {
            return Err(invalid("Wasm oracle dependency identity is incomplete"));
        }
    }
    let actual: Vec<_> = bundle
        .tree
        .files
        .iter()
        .filter_map(|file| {
            let path = file.path.strip_prefix("scripts/oracle/")?;
            (path != "manifest.json").then(|| BuildFile {
                path: path.into(),
                bytes: file.bytes,
                digest: file.digest,
            })
        })
        .collect();
    if actual != manifest.files
        || !actual
            .iter()
            .any(|file| file.path == "ditherette_bench_oracle.js")
        || !actual
            .iter()
            .any(|file| file.path == "ditherette_bench_oracle_bg.wasm")
    {
        return Err(invalid("Wasm oracle emitted closure differs"));
    }
    Ok(())
}

fn oracle_source(path: &str) -> bool {
    [
        "crates/ditherette-bench-oracle/",
        "crates/ditherette-bench-api/",
        "crates/ditherette-wasm/src/spec/",
        "crates/ditherette-wasm/src/image/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
        || [
            "crates/ditherette-bench/src/verification/identity.rs",
            "scripts/prepare-benchmark-oracle.mjs",
            "scripts/benchmark-oracle-page.mjs",
            "scripts/benchmark-indexed-wire.mjs",
            "tools/spec-freeze/checkpoint.json",
            "tools/spec-freeze/dependencies.json",
            "tools/spec-freeze/content.mjs",
            "tools/spec-freeze/build.mjs",
        ]
        .contains(&path)
}
