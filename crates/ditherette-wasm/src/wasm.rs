//! Wasm-facing exports for the fresh crate.
//!
//! These exports are intentionally staged while the Rust-side prod pipeline is
//! wired into the app. Public UI code should prefer coarse pipeline exports once
//! `processRgba8` exists, but staged exports are useful for lazy materialization,
//! memoization, and browser/Wasm benchmarks.

use wasm_bindgen::prelude::*;

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        color::{rgba8_to_color_space_f32, ColorSpaceF32},
        resize::scalar::{
            area::resize_area_rgba8_into,
            bicubic::resize_bicubic_rgba8_into,
            bilinear::{
                alignment::ResizeAnchor as BilinearResizeAnchor, resize_bilinear_rgba8_into,
            },
            convolution::{ResizeAnchor as ConvolutionResizeAnchor, SupportPolicy},
            lanczos::{resize_lanczos2_rgba8_into, resize_lanczos3_rgba8_into},
            nearest::{alignment::ResizeAnchor as NearestResizeAnchor, resize_nearest_rgba8_into},
        },
    },
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

    let dimensions = image_dimensions(width, height)?;
    assert_input_len(input, dimensions)?;

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

/// Resize an RGBA8/sRGB image with the production scalar resize kernels.
///
/// `parallelization_policy` is accepted for API stability; this scalar
/// checkpoint ignores it until production tiling policy exists.
#[wasm_bindgen(js_name = resizeRgba8)]
pub fn resize_rgba8(
    input: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    filter: &str,
    anchor: &str,
    support_policy: &str,
    parallelization_policy: bool,
) -> Result<Vec<u8>, JsValue> {
    let source_dimensions = image_dimensions(source_width, source_height)?;
    let output_dimensions = image_dimensions(output_width, output_height)?;
    assert_input_len(input, source_dimensions)?;

    let source = ImageView::<Rgba8>::packed(input, source_dimensions)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let output_view = ImageViewMut::<Rgba8>::packed(&mut output, output_dimensions)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    let _ = parallelization_policy;
    match filter {
        "nearest" => resize_nearest_rgba8_into(source, output_view, nearest_anchor(anchor)?),
        "area" => resize_area_rgba8_into(source, output_view),
        "bilinear" => resize_bilinear_rgba8_into(source, output_view, bilinear_anchor(anchor)?),
        "bicubic" | "bicubic-catmull-rom" => resize_bicubic_rgba8_into(
            source,
            output_view,
            convolution_anchor(anchor)?,
            parse_support_policy(support_policy)?,
        ),
        "lanczos2" => resize_lanczos2_rgba8_into(
            source,
            output_view,
            convolution_anchor(anchor)?,
            parse_support_policy(support_policy)?,
        ),
        "lanczos3" => resize_lanczos3_rgba8_into(
            source,
            output_view,
            convolution_anchor(anchor)?,
            parse_support_policy(support_policy)?,
        ),
        _ => return Err(JsValue::from_str("unsupported resize filter")),
    }

    Ok(output)
}

fn image_dimensions(width: u32, height: u32) -> Result<ImageDimensions, JsValue> {
    ImageDimensions::new(width, height).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn assert_input_len(input: &[u8], dimensions: ImageDimensions) -> Result<(), JsValue> {
    let expected_len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    if input.len() != expected_len {
        return Err(JsValue::from_str(
            "input length does not match RGBA8 dimensions",
        ));
    }
    Ok(())
}

fn parse_support_policy(value: &str) -> Result<SupportPolicy, JsValue> {
    match value {
        "" | "fixed" => Ok(SupportPolicy::Fixed),
        "scale-aware" | "scale_aware" => Ok(SupportPolicy::ScaleAware),
        _ => Err(JsValue::from_str("unsupported support policy")),
    }
}

fn nearest_anchor(value: &str) -> Result<NearestResizeAnchor, JsValue> {
    match value {
        "" | "center" => Ok(NearestResizeAnchor::Center),
        "top-left" => Ok(NearestResizeAnchor::TopLeft),
        "top" => Ok(NearestResizeAnchor::Top),
        "top-right" => Ok(NearestResizeAnchor::TopRight),
        "left" => Ok(NearestResizeAnchor::Left),
        "right" => Ok(NearestResizeAnchor::Right),
        "bottom-left" => Ok(NearestResizeAnchor::BottomLeft),
        "bottom" => Ok(NearestResizeAnchor::Bottom),
        "bottom-right" => Ok(NearestResizeAnchor::BottomRight),
        _ => Err(JsValue::from_str("unsupported resize anchor")),
    }
}

fn bilinear_anchor(value: &str) -> Result<BilinearResizeAnchor, JsValue> {
    match value {
        "" | "center" => Ok(BilinearResizeAnchor::Center),
        "top-left" => Ok(BilinearResizeAnchor::TopLeft),
        "top" => Ok(BilinearResizeAnchor::Top),
        "top-right" => Ok(BilinearResizeAnchor::TopRight),
        "left" => Ok(BilinearResizeAnchor::Left),
        "right" => Ok(BilinearResizeAnchor::Right),
        "bottom-left" => Ok(BilinearResizeAnchor::BottomLeft),
        "bottom" => Ok(BilinearResizeAnchor::Bottom),
        "bottom-right" => Ok(BilinearResizeAnchor::BottomRight),
        _ => Err(JsValue::from_str("unsupported resize anchor")),
    }
}

fn convolution_anchor(value: &str) -> Result<ConvolutionResizeAnchor, JsValue> {
    match value {
        "" | "center" => Ok(ConvolutionResizeAnchor::Center),
        "top-left" => Ok(ConvolutionResizeAnchor::TopLeft),
        "top" => Ok(ConvolutionResizeAnchor::Top),
        "top-right" => Ok(ConvolutionResizeAnchor::TopRight),
        "left" => Ok(ConvolutionResizeAnchor::Left),
        "right" => Ok(ConvolutionResizeAnchor::Right),
        "bottom-left" => Ok(ConvolutionResizeAnchor::BottomLeft),
        "bottom" => Ok(ConvolutionResizeAnchor::Bottom),
        "bottom-right" => Ok(ConvolutionResizeAnchor::BottomRight),
        _ => Err(JsValue::from_str("unsupported resize anchor")),
    }
}
