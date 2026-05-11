use wasm_bindgen::prelude::*;

use crate::{image::ImageDimensions, resize};

/// Returns a friendly greeting from the Rust/Wasm module.
#[wasm_bindgen]
pub fn hello(name: &str) -> String {
    format!("Hello, {name}, from Ditherette's Rust core!")
}

/// Resizes canonical RGBA bytes with nearest-neighbor sampling.
///
/// JavaScript callers must pass tightly packed, row-major RGBA bytes in browser
/// `ImageData` channel order. The byte length must equal
/// `source_width * source_height * 4`.
///
/// The output is a newly allocated tightly packed RGBA byte vector with length
/// `output_width * output_height * 4`.
#[wasm_bindgen]
pub fn resize_rgba_nearest(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    let source_dimensions =
        ImageDimensions::new(source_width, source_height).map_err(to_js_error)?;
    let output_dimensions =
        ImageDimensions::new(output_width, output_height).map_err(to_js_error)?;

    resize::resize_rgba_nearest(source_rgba, source_dimensions, output_dimensions)
        .map_err(to_js_error)
}

fn to_js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
