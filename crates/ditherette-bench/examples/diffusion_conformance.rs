//! Export the complete public diffusion fixtures with independently bound native identities.

use ditherette_bench::paired::{
    browser::PublicOperation, diffusion::DiffusionSettings, quantize::QuantizeSettings,
};
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::{
    bench_subjects::{self, BenchSubject},
    spec::contract::request::DitherPolicy,
};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        return Err(io::Error::other("usage: diffusion_conformance output.json"));
    };
    let original: serde_json::Value = serde_json::from_str(include_str!(
        "../../../packages/ditherette/tests/fixtures/diffusion.json"
    ))
    .map_err(io::Error::other)?;
    let source = Dimensions {
        width: original["source"]["width"].as_u64().unwrap() as u32,
        height: original["source"]["height"].as_u64().unwrap() as u32,
    };
    let rgba: Vec<u8> =
        serde_json::from_value(original["source"]["data"].clone()).map_err(io::Error::other)?;
    let palette: Vec<ditherette_bench::paired::quantize::PaletteEntry> =
        serde_json::from_value(original["palette"].clone()).map_err(io::Error::other)?;
    let registry = bench_subjects::bench_subjects();
    let mut fixtures = Vec::new();
    for (index, case) in original["cases"].as_array().unwrap().iter().enumerate() {
        let DitherPolicy::Diffusion {
            kernel,
            feedback,
            strength,
            serpentine,
            placement,
        } = serde_json::from_value(case["dither"].clone()).map_err(io::Error::other)?
        else {
            return Err(io::Error::other("expected a diffusion fixture"));
        };
        let operation = PublicOperation::Diffusion {
            settings: DiffusionSettings {
                quantize: QuantizeSettings {
                    palette: palette.clone(),
                    alpha: serde_json::from_value(case["alpha"].clone())
                        .map_err(io::Error::other)?,
                    matching: serde_json::from_value(case["matching"].clone())
                        .map_err(io::Error::other)?,
                },
                kernel,
                feedback,
                strength,
                serpentine,
                placement,
            },
        };
        let request = operation
            .processing_request(source, &rgba)?
            .expect("diffusion request");
        let BenchSubject::Conformance(reference) = registry
            .iter()
            .find(|subject| subject.descriptor().id.as_str() == operation.reference_subject())
            .expect("registered frozen reference")
        else {
            unreachable!()
        };
        let reference = (reference.run)(&request).map_err(io::Error::other)?;
        let encoded = serde_json::to_value(&reference).map_err(io::Error::other)?;
        assert_eq!(encoded["pixels"]["indices"], case["indices"]);
        assert_eq!(encoded["warnings"], case["warnings"]);
        fixtures.push(serde_json::json!({
            "identity": operation.identity(source, &rgba, source)?,
            "name": format!("diffusion-{index}"),
            "operation": operation,
            "source": source,
            "rgba": rgba,
            "reference": reference,
        }));
    }
    assert_eq!(fixtures.len(), 360);
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&serde_json::to_vec_pretty(&fixtures).map_err(io::Error::other)?)
}
