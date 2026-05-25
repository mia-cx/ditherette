//! Wasm-facing exports for the fresh crate.
//!
//! These exports are intentionally staged while the Rust-side prod pipeline is
//! wired into the app. Public UI code should prefer coarse pipeline exports once
//! `processRgba8` exists, but staged exports are useful for lazy materialization,
//! memoization, and browser/Wasm benchmarks.

use wasm_bindgen::prelude::*;

use crate::{
    image::{ImageDimensions, ImageView, Rgba8},
    prod::color::{rgba8_to_color_space_f32, ColorSpaceF32},
};

/// Returns a friendly greeting from the fresh Rust/Wasm module.
#[wasm_bindgen]
pub fn hello(name: &str) -> String {
    format!("Hello, {name}, from Ditherette's fresh Rust core!")
}

/// Materialize an RGBA8/sRGB image into a four-channel f32 color-space buffer.
///
/// `from` currently accepts `rgba8`, `srgb-rgba8`, or `srgb`. `to` accepts the
/// prod f32 color-space names such as `oklab-f32`, `cielab-f32`, and
/// `linear-srgb-f32`. `parallelization_policy` is accepted for API stability;
/// this scalar checkpoint ignores it until the Wasm-thread prototype lands.
#[wasm_bindgen(js_name = convertColorSpace)]
pub fn convert_color_space(
    input: &[u8],
    width: u32,
    height: u32,
    from: &str,
    to: &str,
    parallelization_policy: bool,
) -> Result<Vec<f32>, JsValue> {
    if !matches!(from, "rgba8" | "srgb-rgba8" | "srgb") {
        return Err(JsValue::from_str("unsupported source color space"));
    }

    let dimensions = ImageDimensions::new(width, height)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let expected_len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    if input.len() != expected_len {
        return Err(JsValue::from_str(
            "input length does not match RGBA8 dimensions",
        ));
    }

    let target = ColorSpaceF32::parse(to)
        .ok_or_else(|| JsValue::from_str("unsupported target color space"))?;
    let source = ImageView::<Rgba8>::packed(input, dimensions)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    Ok(rgba8_to_color_space_f32(
        source,
        target,
        parallelization_policy,
    ))
}
