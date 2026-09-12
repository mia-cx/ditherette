use wasm_bindgen::prelude::*;

use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{self, nearest},
};

type AllocatingResizeVariant =
    fn(&[u8], ImageDimensions, ImageDimensions) -> Result<Vec<u8>, ProcessingError>;

/// Returns a friendly greeting from the Rust/Wasm module.
#[wasm_bindgen]
pub fn hello(name: &str) -> String {
    format!("Hello, {name}, from Ditherette's Rust core!")
}

/// Resizes canonical RGBA bytes with nearest-neighbor sampling.
#[wasm_bindgen]
pub fn resize_rgba_nearest(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    resize_with_allocating_variant(
        resize::resize_rgba_nearest,
        source_rgba,
        source_width,
        source_height,
        output_width,
        output_height,
    )
}

/// Resizes canonical RGBA bytes into a JavaScript-provided output buffer.
#[wasm_bindgen]
pub fn resize_rgba_nearest_into(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    output_rgba: &mut [u8],
) -> Result<(), JsValue> {
    let (source_dimensions, output_dimensions) =
        resize_dimensions(source_width, source_height, output_width, output_height)?;

    resize::resize_rgba_nearest_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
    .map_err(to_js_error)
}

/// Resizes canonical RGBA bytes with bilinear interpolation.
#[wasm_bindgen]
pub fn resize_rgba_bilinear(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    resize_with_allocating_variant(
        resize::resize_rgba_bilinear,
        source_rgba,
        source_width,
        source_height,
        output_width,
        output_height,
    )
}

/// Resizes canonical RGBA bytes into a JavaScript-provided output buffer.
#[wasm_bindgen]
pub fn resize_rgba_bilinear_into(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    output_rgba: &mut [u8],
) -> Result<(), JsValue> {
    let (source_dimensions, output_dimensions) =
        resize_dimensions(source_width, source_height, output_width, output_height)?;

    resize::resize_rgba_bilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
    .map_err(to_js_error)
}

/// Resizes canonical RGBA bytes with mipmapped trilinear interpolation.
#[wasm_bindgen]
pub fn resize_rgba_trilinear(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    resize_with_allocating_variant(
        resize::resize_rgba_trilinear,
        source_rgba,
        source_width,
        source_height,
        output_width,
        output_height,
    )
}

/// Resizes canonical RGBA bytes with exact area averaging.
#[wasm_bindgen]
pub fn resize_rgba_area(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    resize_with_allocating_variant(
        resize::resize_rgba_area,
        source_rgba,
        source_width,
        source_height,
        output_width,
        output_height,
    )
}

/// Alias for exact area averaging using box-filter terminology.
#[wasm_bindgen]
pub fn resize_rgba_box(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    resize_with_allocating_variant(
        resize::resize_rgba_box,
        source_rgba,
        source_width,
        source_height,
        output_width,
        output_height,
    )
}

/// Applies a 3x3 box post-resize antialiasing pass.
#[wasm_bindgen]
pub fn antialias_rgba_box3(
    source_rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, JsValue> {
    let dimensions = ImageDimensions::new(width, height).map_err(to_js_error)?;
    resize::antialias_rgba_box3(source_rgba, dimensions).map_err(to_js_error)
}

/// Benchmark reference variant that uses the straightforward readable loop.
#[wasm_bindgen]
pub fn resize_rgba_nearest_reference(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    resize_with_allocating_variant(
        nearest::resize_rgba_nearest_reference,
        source_rgba,
        source_width,
        source_height,
        output_width,
        output_height,
    )
}

fn resize_with_allocating_variant(
    variant: AllocatingResizeVariant,
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue> {
    let (source_dimensions, output_dimensions) =
        resize_dimensions(source_width, source_height, output_width, output_height)?;

    variant(source_rgba, source_dimensions, output_dimensions).map_err(to_js_error)
}

fn resize_dimensions(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<(ImageDimensions, ImageDimensions), JsValue> {
    let source_dimensions =
        ImageDimensions::new(source_width, source_height).map_err(to_js_error)?;
    let output_dimensions =
        ImageDimensions::new(output_width, output_height).map_err(to_js_error)?;
    Ok((source_dimensions, output_dimensions))
}

fn to_js_error(error: ProcessingError) -> JsValue {
    JsValue::from_str(&error.to_string())
}
