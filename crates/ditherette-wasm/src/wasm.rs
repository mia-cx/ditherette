//! Wasm-facing exports for the fresh crate.
//!
//! These exports are intentionally staged while the Rust-side prod pipeline is
//! wired into the app. Public UI code should prefer coarse pipeline exports once
//! `processRgba8` exists, but staged exports are useful for lazy materialization,
//! memoization, and browser/Wasm benchmarks.

use std::hint::black_box;

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
    let resize = WasmResize::parse(filter, anchor, support_policy)?;
    let mut output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

    let _ = parallelization_policy;
    resize.run(source, output_dimensions, &mut output)?;

    Ok(output)
}

/// Benchmark an RGBA8 resize entirely inside Wasm and return a JSON result.
///
/// Each sample times a calibrated batch of resize kernel executions and reports
/// per-resize nanoseconds. The JS harness only decodes fixtures and renders
/// results; the sub-millisecond timing loop lives here.
#[wasm_bindgen(js_name = benchmarkResizeRgba8)]
pub fn benchmark_resize_rgba8(
    input: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    filter: &str,
    anchor: &str,
    support_policy: &str,
    parallelization_policy: bool,
    sample_size: u32,
    warm_up_iterations: u32,
    target_sample_time_ms: f64,
) -> Result<String, JsValue> {
    let source_dimensions = image_dimensions(source_width, source_height)?;
    let output_dimensions = image_dimensions(output_width, output_height)?;
    assert_input_len(input, source_dimensions)?;
    if sample_size == 0 {
        return Err(JsValue::from_str("sample size must be greater than zero"));
    }

    let source = ImageView::<Rgba8>::packed(input, source_dimensions)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let resize = WasmResize::parse(filter, anchor, support_policy)?;
    let mut output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

    let _ = parallelization_policy;
    for _ in 0..warm_up_iterations {
        resize.run(source, output_dimensions, &mut output)?;
        black_box(output.as_slice());
    }

    let batch_size = calibrated_batch_size(
        source,
        output_dimensions,
        &mut output,
        resize,
        target_sample_time_ms,
    )?;
    let mut samples_ns = Vec::with_capacity(sample_size as usize);
    for _ in 0..sample_size {
        let started = performance_now();
        for _ in 0..batch_size {
            resize.run(source, output_dimensions, &mut output)?;
            black_box(output.as_slice());
        }
        let elapsed_ms = performance_now() - started;
        samples_ns.push(elapsed_ms.max(0.0) * 1_000_000.0 / f64::from(batch_size));
    }

    Ok(benchmark_result_json(
        batch_size,
        batch_size as u64 * u64::from(sample_size),
        output.len(),
        checksum_bytes(&output),
        &samples_ns,
    ))
}

#[derive(Clone, Copy)]
struct WasmResize {
    filter: WasmResizeFilter,
    nearest_anchor: NearestResizeAnchor,
    bilinear_anchor: BilinearResizeAnchor,
    convolution_anchor: ConvolutionResizeAnchor,
    support_policy: SupportPolicy,
}

#[derive(Clone, Copy)]
enum WasmResizeFilter {
    Nearest,
    Area,
    Bilinear,
    Bicubic,
    Lanczos2,
    Lanczos3,
}

impl WasmResize {
    fn parse(filter: &str, anchor: &str, support_policy: &str) -> Result<Self, JsValue> {
        let filter = match filter {
            "nearest" => WasmResizeFilter::Nearest,
            "area" => WasmResizeFilter::Area,
            "bilinear" => WasmResizeFilter::Bilinear,
            "bicubic" | "bicubic-catmull-rom" => WasmResizeFilter::Bicubic,
            "lanczos2" => WasmResizeFilter::Lanczos2,
            "lanczos3" => WasmResizeFilter::Lanczos3,
            _ => return Err(JsValue::from_str("unsupported resize filter")),
        };

        Ok(Self {
            filter,
            nearest_anchor: nearest_anchor(anchor)?,
            bilinear_anchor: bilinear_anchor(anchor)?,
            convolution_anchor: convolution_anchor(anchor)?,
            support_policy: parse_support_policy(support_policy)?,
        })
    }

    fn run(
        self,
        source: ImageView<'_, Rgba8>,
        output_dimensions: ImageDimensions,
        output: &mut [u8],
    ) -> Result<(), JsValue> {
        let output_view = ImageViewMut::<Rgba8>::packed(output, output_dimensions)
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        match self.filter {
            WasmResizeFilter::Nearest => {
                resize_nearest_rgba8_into(source, output_view, self.nearest_anchor)
            }
            WasmResizeFilter::Area => resize_area_rgba8_into(source, output_view),
            WasmResizeFilter::Bilinear => {
                resize_bilinear_rgba8_into(source, output_view, self.bilinear_anchor)
            }
            WasmResizeFilter::Bicubic => resize_bicubic_rgba8_into(
                source,
                output_view,
                self.convolution_anchor,
                self.support_policy,
            ),
            WasmResizeFilter::Lanczos2 => resize_lanczos2_rgba8_into(
                source,
                output_view,
                self.convolution_anchor,
                self.support_policy,
            ),
            WasmResizeFilter::Lanczos3 => resize_lanczos3_rgba8_into(
                source,
                output_view,
                self.convolution_anchor,
                self.support_policy,
            ),
        }
        Ok(())
    }
}

fn calibrated_batch_size(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    resize: WasmResize,
    target_sample_time_ms: f64,
) -> Result<u32, JsValue> {
    const MAX_BATCH_SIZE: u32 = 1 << 20;

    let target_sample_time_ms = if target_sample_time_ms.is_finite() && target_sample_time_ms > 0.0
    {
        target_sample_time_ms
    } else {
        1.0
    };
    let mut batch_size = 1;
    loop {
        let started = performance_now();
        for _ in 0..batch_size {
            resize.run(source, output_dimensions, output)?;
            black_box(output.as_ref());
        }
        let elapsed = performance_now() - started;
        if elapsed >= target_sample_time_ms || batch_size >= MAX_BATCH_SIZE {
            return Ok(batch_size);
        }
        let multiplier = (target_sample_time_ms / elapsed.max(0.001)).ceil() as u32;
        batch_size = batch_size
            .saturating_mul(multiplier.max(2))
            .min(MAX_BATCH_SIZE);
    }
}

fn benchmark_result_json(
    batch_size: u32,
    total_iterations: u64,
    output_byte_length: usize,
    checksum: u32,
    samples_ns: &[f64],
) -> String {
    let samples = samples_ns
        .iter()
        .map(|sample| format!("{sample:.0}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"batchSize\":{batch_size},\"totalIterations\":{total_iterations},\"outputByteLength\":{output_byte_length},\"checksum\":{checksum},\"samplesNs\":[{samples}]}}"
    )
}

fn checksum_bytes(bytes: &[u8]) -> u32 {
    let mut hash = 2_166_136_261u32;
    for &byte in bytes {
        hash ^= u32::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;
}

#[cfg(not(target_arch = "wasm32"))]
fn performance_now() -> f64 {
    use std::sync::OnceLock;
    use std::time::Instant;

    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1_000.0
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
