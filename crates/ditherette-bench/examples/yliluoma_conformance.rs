//! Full semantic identities for the 367 retained Yliluoma fixtures. No measurements run.
use ditherette_bench::paired::{
    browser::PublicOperation, quantize::QuantizeSettings, yliluoma::YliluomaSettings,
};
use ditherette_bench_api::verification::Dimensions;
use ditherette_bench_oracle::OracleRequest;
use serde_json::{json, Value};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
};

fn fixtures() -> io::Result<Vec<Value>> {
    let native: Value = serde_json::from_str(include_str!(
        "../../../packages/ditherette/tests/fixtures/yiluoma.json"
    ))
    .map_err(io::Error::other)?;
    let mut fixtures = Vec::new();
    for case in native["cases"].as_array().unwrap() {
        let raw = &case["request"];
        let source = Dimensions {
            width: raw["source"]["width"].as_u64().unwrap() as u32,
            height: raw["source"]["height"].as_u64().unwrap() as u32,
        };
        let rgba: Vec<u8> =
            serde_json::from_value(raw["source"]["data"].clone()).map_err(io::Error::other)?;
        let settings = YliluomaSettings {
            quantize: serde_json::from_value::<QuantizeSettings>(
                json!({"palette":raw["palette"],"alpha":raw["alpha"],"matching":raw["matching"]}),
            )
            .map_err(io::Error::other)?,
            size: serde_json::from_value(raw["dither"]["size"].clone())
                .map_err(io::Error::other)?,
            placement: serde_json::from_value(raw["dither"]["placement"].clone())
                .map_err(io::Error::other)?,
        };
        let operation = PublicOperation::Yliluoma { settings };
        let identity = operation.identity(source, &rgba, source)?;
        let oracle: OracleRequest = serde_json::from_value(json!({"source":source,"rgba":rgba,"output":source,"operation":operation,"identity":identity})).map_err(io::Error::other)?;
        let reference = oracle.execute().map_err(io::Error::other)?;
        let expected = &case["output"];
        assert_eq!(
            serde_json::to_value(&reference.output).unwrap(),
            json!({"dimensions":source,"pixels":{"format":"indexed8","indices":expected["indices"],"palette_rgba":expected["palette"]["rgba"],"transparent_index":expected["palette"]["transparentIndex"]},"warnings":expected["warnings"]})
        );
        fixtures.push(json!({"identity":identity,"operation":operation,"source":source,"rgba":rgba,"reference":reference.output}));
    }
    Ok(fixtures)
}
fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        return Err(io::Error::other(
            "usage: yliluoma_conformance NEW_OUTPUT.json",
        ));
    };
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?
        .write_all(&serde_json::to_vec(&fixtures()?).map_err(io::Error::other)?)
}
#[cfg(test)]
mod tests {
    #[test]
    fn all_retained_requests_bind_independent_oracle_identity_and_native_output() {
        assert_eq!(super::fixtures().unwrap().len(), 367);
    }
}
