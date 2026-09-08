//! Independent target-local oracle. Only frozen image and spec modules are linked.
#[path = "/home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/crates/ditherette-wasm/src/image/mod.rs"]
pub mod image;
#[path = "/home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/crates/ditherette-wasm/src/spec/mod.rs"]
pub mod spec;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn fixture(json: &str) -> String {
    use spec::contract::request::*;
    let mut fixtures: serde_json::Value = serde_json::from_str(json).unwrap();
    for case in fixtures["cases"].as_array_mut().unwrap() {
        let raw = &case["request"];
        let data: Vec<u8> = serde_json::from_value(raw["source"]["data"].clone()).unwrap();
        let palette: Vec<image::contracts::PaletteEntry> = serde_json::from_value(raw["palette"].clone()).unwrap();
        let request = DitherQuantizeRequest {quantize: QuantizeRequest {version:1,source: Source {width:raw["source"]["width"].as_u64().unwrap() as u32,height:raw["source"]["height"].as_u64().unwrap() as u32,data:&data},palette:&palette,matching:serde_json::from_value(raw["matching"].clone()).unwrap(),alpha:serde_json::from_value(raw["alpha"].clone()).unwrap()},dither:serde_json::from_value(raw["dither"].clone()).unwrap()};
        let output = spec::dither::yiluoma::dither_yiluoma(request).unwrap();
        case["output"] = serde_json::json!({"width":request.quantize.source.width,"height":request.quantize.source.height,"indices":output.indices.data(),"palette":{"rgba":output.palette.rgba,"transparentIndex":output.palette.transparent_index},"warnings":output.warnings});
    }
    fixtures.to_string()
}
