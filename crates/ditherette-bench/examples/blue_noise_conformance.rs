//! Frozen outputs for actual installed-package S27 benchmark adapters, without timing.
#[path = "support/blue_noise.rs"]
mod blue_noise;
use ditherette_wasm::bench_subjects::{self, BenchSubject};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        return Err(io::Error::other("usage: blue_noise_conformance NEW_JSON"));
    };
    let (source, rgba) = blue_noise::fixture();
    let registry = bench_subjects::bench_subjects();
    let mut fixtures = Vec::new();
    for (name, native) in blue_noise::recipes() {
        let request = native.reference_request(source, &rgba)?;
        let BenchSubject::Conformance(subject) = registry
            .iter()
            .find(|subject| subject.descriptor().id.as_str() == native.reference_subject())
            .expect("frozen reference")
        else {
            unreachable!()
        };
        let reference = (subject.run)(&request).map_err(io::Error::other)?;
        fixtures.push(serde_json::json!({ "name": name, "operation": blue_noise::public(&native), "source": source, "rgba": rgba, "reference": reference, "identity": native.identity(source, &rgba)? }));
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(&serde_json::to_vec_pretty(&fixtures).map_err(io::Error::other)?)
}
