//! Wasm-facing exports for the fresh crate.
//!
//! These exports are intentionally staged while the Rust-side prod pipeline is
//! wired into the app. Public UI code should prefer coarse pipeline exports once
//! `processRgba8` exists, but staged exports are useful for lazy materialization,
//! memoization, and browser/Wasm benchmarks.

pub mod processor;

use std::{hint::black_box, num::NonZeroU32};

use js_sys::Function;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use crate::{
    image::{ImageDimensions, ImageFormat, ImageView, ImageViewMut, Rgba8},
    prod::{
        color::{
            rgba8_to_color_space_f32, rgba8_to_color_space_f32_with_policy_into, ColorSpaceF32,
        },
        resize::scalar::{
            area::resize_area_rgba8_into,
            bicubic::{
                resize_bicubic_rgba8_into, resize_bicubic_rgba8_rows_into,
                resize_bicubic_rgba8_rows_with_plan_into, BicubicResizePlan,
            },
            bilinear::{
                alignment::ResizeAnchor as BilinearResizeAnchor, resize_bilinear_rgba8_into,
            },
            convolution::{ResizeAnchor as ConvolutionResizeAnchor, SupportPolicy},
            lanczos::{
                resize_lanczos2_rgba8_into, resize_lanczos2_rgba8_rows_into,
                resize_lanczos3_rgba8_into, resize_lanczos3_rgba8_rows_into,
                resize_lanczos_rgba8_rows_with_plan_into, LanczosResizePlan,
            },
            nearest::{
                alignment::ResizeAnchor as NearestResizeAnchor, resize_nearest_rgba8_into,
                resize_nearest_rgba8_rows_with_plan_into, NearestResizePlan,
            },
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
/// `linear-srgb-f32`. When threaded Wasm is enabled,
/// `parallelization_policy` enables production color tiling for images large
/// enough to amortize worker overhead.
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
    resize_rgba8_scalar(
        input,
        source_width,
        source_height,
        output_width,
        output_height,
        filter,
        anchor,
        support_policy,
        parallelization_policy,
    )
}

/// Execute the coarse RGBA8 processing pipeline inside Wasm.
///
/// This is the full-pipeline shell from the parallelization plan. It accepts the
/// app-style serialized processing settings and currently executes the scalar
/// resize stage, returning RGBA8 for preview. Later stages can be inserted here
/// without changing the JS/Wasm boundary shape.
#[wasm_bindgen(js_name = processRgba8)]
pub fn process_rgba8(
    input: &[u8],
    width: u32,
    height: u32,
    settings_json: &str,
    parallelization_policy: bool,
) -> Result<Vec<u8>, JsValue> {
    let settings: ProcessRgba8Settings = serde_json::from_str(settings_json)
        .map_err(|error| JsValue::from_str(&format!("invalid process settings: {error}")))?;
    validate_process_crop(settings.output.crop, width, height)?;
    let resize = resize_request_from_mode(&settings.output.resize)?;

    resize_rgba8_scalar(
        input,
        width,
        height,
        settings.output.width,
        settings.output.height,
        resize.filter,
        CENTER_ANCHOR,
        resize.support_policy,
        parallelization_policy,
    )
}

/// Benchmark color-space materialization entirely inside Wasm and return JSON.
///
/// The `scalar` mode runs the direct scalar row path. The `pooled_direct` mode
/// allows the internal parallelization policy; in the threaded build that maps
/// to the Rayon-backed direct row-write prototype.
#[wasm_bindgen(js_name = benchmarkColorSpace)]
pub fn benchmark_color_space(
    input: &[u8],
    width: u32,
    height: u32,
    to: &str,
    execution_mode: &str,
    sample_size: u32,
    measurement_time_ms: f64,
    warm_up_time_ms: f64,
    warm_up_iterations: u32,
    target_sample_time_ms: f64,
    live_stats: bool,
    row_band_height: u32,
    reporter: Option<Function>,
) -> Result<String, JsValue> {
    let dimensions = image_dimensions(width, height)?;
    assert_input_len(input, dimensions)?;
    if sample_size == 0 {
        return Err(JsValue::from_str("sample size must be greater than zero"));
    }

    let target = ColorSpaceF32::parse(to)
        .ok_or_else(|| JsValue::from_str("unsupported target color space"))?;
    let mode = ColorBenchmarkMode::parse(execution_mode)?;
    let source = ImageView::<Rgba8>::packed(input, dimensions)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let mut output = vec![0.0; dimensions.storage_len::<Rgba8>().unwrap()];
    let config = WasmBenchmarkConfig {
        sample_size,
        measurement_time_ms: positive_or_default(measurement_time_ms, 5_000.0),
        warm_up_time_ms: positive_or_default(warm_up_time_ms, 1_000.0),
        warm_up_iterations,
        target_sample_time_ms: positive_or_default(target_sample_time_ms, 1.0),
        live_stats,
        row_band_height: row_band_height.max(1) as usize,
    };
    let result = run_color_benchmark(source, target, mode, &mut output, config, reporter.as_ref())?;

    Ok(benchmark_result_json(
        result.batch_size,
        result.total_iterations,
        output.len() * std::mem::size_of::<f32>(),
        checksum_f32(&output),
        &result.samples_ns,
    ))
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
    measurement_time_ms: f64,
    warm_up_time_ms: f64,
    warm_up_iterations: u32,
    target_sample_time_ms: f64,
    live_stats: bool,
    row_band_height: u32,
    reporter: Option<Function>,
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

    let config = WasmBenchmarkConfig {
        sample_size,
        measurement_time_ms: positive_or_default(measurement_time_ms, 5_000.0),
        warm_up_time_ms: positive_or_default(warm_up_time_ms, 1_000.0),
        warm_up_iterations,
        target_sample_time_ms: positive_or_default(target_sample_time_ms, 1.0),
        live_stats,
        row_band_height: row_band_height.max(1) as usize,
    };
    let result = run_resize_benchmark(
        source,
        output_dimensions,
        resize,
        parallelization_policy,
        &mut output,
        config,
        reporter.as_ref(),
    )?;

    Ok(benchmark_result_json(
        result.batch_size,
        result.total_iterations,
        output.len(),
        checksum_bytes(&output),
        &result.samples_ns,
    ))
}

const CENTER_ANCHOR: &str = "center";
const FIXED_SUPPORT: &str = "fixed";
const SCALE_AWARE_SUPPORT: &str = "scale-aware";
const PROTOTYPE_ROW_BAND_HEIGHT: usize = 32;
#[allow(dead_code)]
const NEAREST_PARALLEL_PIXEL_THRESHOLD: usize = 40_000;
#[allow(dead_code)]
const NEAREST_MIN_DOWNSCALE_RATIO: f64 = 0.25;
#[allow(dead_code)]
const NEAREST_NEAR_IDENTITY_DOWNSCALE_RATIO: f64 = 0.98;
#[allow(dead_code)]
const NEAREST_ROW_BAND_HEIGHT: usize = 32;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessRgba8Settings {
    output: ProcessOutputSettings,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessOutputSettings {
    width: u32,
    height: u32,
    resize: String,
    crop: Option<ProcessCropRect>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProcessCropRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Clone, Copy)]
struct ProcessResizeRequest {
    filter: &'static str,
    support_policy: &'static str,
}

fn validate_process_crop(
    crop: Option<ProcessCropRect>,
    source_width: u32,
    source_height: u32,
) -> Result<(), JsValue> {
    let Some(crop) = crop else {
        return Ok(());
    };
    if crop.x == 0 && crop.y == 0 && crop.width == source_width && crop.height == source_height {
        return Ok(());
    }
    Err(JsValue::from_str(
        "processRgba8 does not support cropped sources yet",
    ))
}

fn resize_request_from_mode(mode: &str) -> Result<ProcessResizeRequest, JsValue> {
    let request = match mode {
        "nearest" => ProcessResizeRequest {
            filter: "nearest",
            support_policy: FIXED_SUPPORT,
        },
        "area" => ProcessResizeRequest {
            filter: "area",
            support_policy: FIXED_SUPPORT,
        },
        "bilinear" => ProcessResizeRequest {
            filter: "bilinear",
            support_policy: FIXED_SUPPORT,
        },
        "bicubic" | "bicubic-catmull-rom" => ProcessResizeRequest {
            filter: "bicubic",
            support_policy: FIXED_SUPPORT,
        },
        "lanczos2" => ProcessResizeRequest {
            filter: "lanczos2",
            support_policy: FIXED_SUPPORT,
        },
        "lanczos2-scale-aware" => ProcessResizeRequest {
            filter: "lanczos2",
            support_policy: SCALE_AWARE_SUPPORT,
        },
        "lanczos3" => ProcessResizeRequest {
            filter: "lanczos3",
            support_policy: FIXED_SUPPORT,
        },
        "lanczos3-scale-aware" => ProcessResizeRequest {
            filter: "lanczos3",
            support_policy: SCALE_AWARE_SUPPORT,
        },
        _ => return Err(JsValue::from_str("unsupported process resize mode")),
    };
    Ok(request)
}

fn resize_rgba8_scalar(
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

    #[cfg(feature = "threads")]
    if parallelization_policy {
        if let Some(policy) = resize.production_tiling_policy(source_dimensions, output_dimensions)
        {
            resize.run_pooled_direct(
                source,
                output_dimensions,
                &mut output,
                policy.row_band_height(),
            )?;
            return Ok(output);
        }
    }

    if parallelization_policy
        && resize.run_pooled(
            source,
            output_dimensions,
            &mut output,
            PROTOTYPE_ROW_BAND_HEIGHT,
        )?
    {
        return Ok(output);
    }

    resize.run(source, output_dimensions, &mut output)?;

    Ok(output)
}

#[derive(Clone, Copy)]
struct WasmResize {
    filter: WasmResizeFilter,
    nearest_anchor: NearestResizeAnchor,
    bilinear_anchor: BilinearResizeAnchor,
    convolution_anchor: ConvolutionResizeAnchor,
    support_policy: SupportPolicy,
    plan_scope: ResizePlanScope,
    execution_mode: ResizeExecutionMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizeExecutionMode {
    PolicyDefault,
    PooledDirect,
    PooledNoop,
    PooledCopy,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ResizePlanScope {
    FullImage,
    PerBand,
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

        let (support_policy, plan_scope, execution_mode) =
            parse_support_policy_and_execution(support_policy)?;
        Ok(Self {
            filter,
            nearest_anchor: nearest_anchor(anchor)?,
            bilinear_anchor: bilinear_anchor(anchor)?,
            convolution_anchor: convolution_anchor(anchor)?,
            support_policy,
            plan_scope,
            execution_mode,
        })
    }

    fn run_pooled(
        self,
        source: ImageView<'_, Rgba8>,
        output_dimensions: ImageDimensions,
        output: &mut [u8],
        row_band_height: usize,
    ) -> Result<bool, JsValue> {
        match self.execution_mode {
            ResizeExecutionMode::PolicyDefault => Ok(false),
            ResizeExecutionMode::PooledNoop => {
                resize_pooled_noop_into(output_dimensions, output, row_band_height)?;
                Ok(true)
            }
            ResizeExecutionMode::PooledCopy => {
                resize_pooled_copy_into(source, output_dimensions, output, row_band_height)?;
                Ok(true)
            }
            ResizeExecutionMode::PooledDirect => {
                self.run_pooled_direct(source, output_dimensions, output, row_band_height)?;
                Ok(true)
            }
        }
    }

    fn run_pooled_direct(
        self,
        source: ImageView<'_, Rgba8>,
        output_dimensions: ImageDimensions,
        output: &mut [u8],
        row_band_height: usize,
    ) -> Result<(), JsValue> {
        match self.filter {
            WasmResizeFilter::Nearest => resize_nearest_rgba8_pooled_direct_into(
                source,
                output_dimensions,
                output,
                self.nearest_anchor,
                row_band_height,
            ),
            WasmResizeFilter::Bicubic => resize_bicubic_rgba8_pooled_direct_into(
                source,
                output_dimensions,
                output,
                self.convolution_anchor,
                self.support_policy,
                self.plan_scope,
                row_band_height,
            ),
            WasmResizeFilter::Lanczos2 => resize_lanczos_rgba8_pooled_direct_into(
                source,
                output_dimensions,
                output,
                self.convolution_anchor,
                NonZeroU32::new(2).unwrap(),
                self.support_policy,
                self.plan_scope,
                row_band_height,
            ),
            WasmResizeFilter::Lanczos3 => resize_lanczos_rgba8_pooled_direct_into(
                source,
                output_dimensions,
                output,
                self.convolution_anchor,
                NonZeroU32::new(3).unwrap(),
                self.support_policy,
                self.plan_scope,
                row_band_height,
            ),
            _ => Err(JsValue::from_str(
                "resize filter does not support pooled direct",
            )),
        }
    }

    #[allow(dead_code)]
    fn production_tiling_policy(
        self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
    ) -> Option<NearestResizeTilingPolicy> {
        if self.execution_mode != ResizeExecutionMode::PolicyDefault
            || !matches!(self.filter, WasmResizeFilter::Nearest)
            || source_dimensions == output_dimensions
            || nearest_exact_integer_upscale(source_dimensions, output_dimensions)
            || output_dimensions.pixel_count().expect("valid dimensions")
                < NEAREST_PARALLEL_PIXEL_THRESHOLD
        {
            return None;
        }

        let scale_x = output_dimensions.width() as f64 / source_dimensions.width() as f64;
        let scale_y = output_dimensions.height() as f64 / source_dimensions.height() as f64;
        if !nearest_parallel_scale_is_profitable(scale_x, scale_y) {
            return None;
        }

        Some(NearestResizeTilingPolicy {
            row_band_height: NEAREST_ROW_BAND_HEIGHT,
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NearestResizeTilingPolicy {
    row_band_height: usize,
}

#[allow(dead_code)]
impl NearestResizeTilingPolicy {
    const fn row_band_height(self) -> usize {
        self.row_band_height
    }
}

#[allow(dead_code)]
fn nearest_parallel_scale_is_profitable(scale_x: f64, scale_y: f64) -> bool {
    if scale_x <= 0.0 || scale_y <= 0.0 {
        return false;
    }

    if scale_x <= 1.0 && scale_y <= 1.0 {
        let min_scale = scale_x.min(scale_y);
        let max_scale = scale_x.max(scale_y);
        return min_scale >= NEAREST_MIN_DOWNSCALE_RATIO
            && max_scale <= NEAREST_NEAR_IDENTITY_DOWNSCALE_RATIO;
    }

    scale_x >= 1.0 && scale_y >= 1.0
}

fn nearest_exact_integer_upscale(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    source_dimensions.width() < output_dimensions.width()
        && source_dimensions.height() < output_dimensions.height()
        && output_dimensions.width() % source_dimensions.width() == 0
        && output_dimensions.height() % source_dimensions.height() == 0
}

fn resize_pooled_noop_into(
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    row_band_height: usize,
) -> Result<(), JsValue> {
    resize_rows_pooled_direct_into(
        output_dimensions,
        output,
        row_band_height,
        |output_view, _y_start| {
            black_box(output_view.data().as_ptr());
        },
    )
}

fn resize_pooled_copy_into(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    row_band_height: usize,
) -> Result<(), JsValue> {
    resize_rows_pooled_direct_into(
        output_dimensions,
        output,
        row_band_height,
        |mut output_view, y_start| {
            let source = source.data();
            let output = output_view.data_mut();
            let source_offset = (y_start as usize * output.len()) % source.len();
            for (index, byte) in output.iter_mut().enumerate() {
                *byte = source[(source_offset + index) % source.len()];
            }
        },
    )
}

fn resize_bicubic_rgba8_pooled_direct_into(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    anchor: ConvolutionResizeAnchor,
    support_policy: SupportPolicy,
    plan_scope: ResizePlanScope,
    row_band_height: usize,
) -> Result<(), JsValue> {
    if plan_scope == ResizePlanScope::PerBand {
        return resize_rows_pooled_direct_into(
            output_dimensions,
            output,
            row_band_height,
            |output_view, y_start| {
                resize_bicubic_rgba8_rows_into(
                    source,
                    output_view,
                    output_dimensions,
                    y_start,
                    anchor,
                    support_policy,
                );
            },
        );
    }

    let plan = BicubicResizePlan::new(
        source.dimensions(),
        output_dimensions,
        anchor,
        support_policy,
    );
    resize_rows_pooled_direct_into(
        output_dimensions,
        output,
        row_band_height,
        |output_view, y_start| {
            resize_bicubic_rgba8_rows_with_plan_into(source, output_view, &plan, y_start);
        },
    )
}

fn resize_lanczos_rgba8_pooled_direct_into(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    anchor: ConvolutionResizeAnchor,
    radius: NonZeroU32,
    support_policy: SupportPolicy,
    plan_scope: ResizePlanScope,
    row_band_height: usize,
) -> Result<(), JsValue> {
    if plan_scope == ResizePlanScope::PerBand {
        return resize_rows_pooled_direct_into(
            output_dimensions,
            output,
            row_band_height,
            |output_view, y_start| match radius.get() {
                2 => resize_lanczos2_rgba8_rows_into(
                    source,
                    output_view,
                    output_dimensions,
                    y_start,
                    anchor,
                    support_policy,
                ),
                3 => resize_lanczos3_rgba8_rows_into(
                    source,
                    output_view,
                    output_dimensions,
                    y_start,
                    anchor,
                    support_policy,
                ),
                _ => unreachable!("only Lanczos2/3 pooled probes are wired"),
            },
        );
    }

    let plan = LanczosResizePlan::new(
        source.dimensions(),
        output_dimensions,
        anchor,
        radius,
        support_policy,
    );
    resize_rows_pooled_direct_into(
        output_dimensions,
        output,
        row_band_height,
        |output_view, y_start| {
            resize_lanczos_rgba8_rows_with_plan_into(source, output_view, &plan, y_start);
        },
    )
}

fn resize_nearest_rgba8_pooled_direct_into(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    anchor: NearestResizeAnchor,
    row_band_height: usize,
) -> Result<(), JsValue> {
    let plan = NearestResizePlan::new(source.dimensions(), output_dimensions, anchor);
    resize_rows_pooled_direct_into(
        output_dimensions,
        output,
        row_band_height,
        |output_view, y_start| {
            resize_nearest_rgba8_rows_with_plan_into(source, output_view, &plan, y_start);
        },
    )
}

fn resize_rows_pooled_direct_into(
    output_dimensions: ImageDimensions,
    output: &mut [u8],
    row_band_height: usize,
    process_row: impl Fn(ImageViewMut<'_, Rgba8>, u32) + Sync,
) -> Result<(), JsValue> {
    let row_band_height = row_band_height.max(1);
    let row_len = output_dimensions.width_usize() * Rgba8::CHANNEL_COUNT;
    let band_len = row_len * row_band_height;

    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        output
            .par_chunks_mut(band_len)
            .enumerate()
            .for_each(|(band_index, output_band)| {
                let y_start = (band_index * row_band_height) as u32;
                let band_height = (output_band.len() / row_len) as u32;
                let band_dimensions = ImageDimensions::new(output_dimensions.width(), band_height)
                    .expect("band dimensions should be valid");
                let output_view = ImageViewMut::<Rgba8>::packed(output_band, band_dimensions)
                    .expect("band output view should be valid");
                process_row(output_view, y_start);
            });
    }

    #[cfg(not(feature = "threads"))]
    {
        for (band_index, output_band) in output.chunks_mut(band_len).enumerate() {
            let y_start = (band_index * row_band_height) as u32;
            let band_height = (output_band.len() / row_len) as u32;
            let band_dimensions = ImageDimensions::new(output_dimensions.width(), band_height)
                .map_err(|error| JsValue::from_str(&error.to_string()))?;
            let output_view = ImageViewMut::<Rgba8>::packed(output_band, band_dimensions)
                .map_err(|error| JsValue::from_str(&error.to_string()))?;
            process_row(output_view, y_start);
        }
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct WasmBenchmarkConfig {
    sample_size: u32,
    measurement_time_ms: f64,
    warm_up_time_ms: f64,
    warm_up_iterations: u32,
    target_sample_time_ms: f64,
    live_stats: bool,
    row_band_height: usize,
}

struct WasmBenchmarkResult {
    batch_size: u32,
    total_iterations: u64,
    samples_ns: Vec<f64>,
}

#[derive(Clone, Copy)]
enum ColorBenchmarkMode {
    Scalar,
    PooledDirect,
    PooledNoop,
    PooledCopy,
}

impl ColorBenchmarkMode {
    fn parse(value: &str) -> Result<Self, JsValue> {
        match value {
            "scalar" => Ok(Self::Scalar),
            "pooled_direct" => Ok(Self::PooledDirect),
            "pooled_noop" => Ok(Self::PooledNoop),
            "pooled_copy" => Ok(Self::PooledCopy),
            _ => Err(JsValue::from_str(
                "unsupported color benchmark execution mode",
            )),
        }
    }
}

fn run_color_benchmark(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    mode: ColorBenchmarkMode,
    output: &mut [f32],
    config: WasmBenchmarkConfig,
    reporter: Option<&Function>,
) -> Result<WasmBenchmarkResult, JsValue> {
    const MAX_BATCH_SIZE: u32 = 1 << 20;

    let mut batch_size = 1;
    let warmup_started = performance_now();
    let mut warmup_elapsed = 0.0;
    let mut warmup_batches = 0;
    let mut warmup_iterations = 0u64;
    let mut best_batch_size = batch_size;
    let mut best_batch_elapsed = f64::INFINITY;
    while warmup_elapsed < config.warm_up_time_ms
        || (config.warm_up_iterations > 0 && warmup_batches < config.warm_up_iterations)
    {
        let elapsed = run_color_batch(
            source,
            target,
            mode,
            output,
            batch_size,
            config.row_band_height,
        );
        if (elapsed - config.target_sample_time_ms).abs()
            < (best_batch_elapsed - config.target_sample_time_ms).abs()
        {
            best_batch_size = batch_size;
            best_batch_elapsed = elapsed;
        }
        warmup_batches += 1;
        warmup_iterations += u64::from(batch_size);
        warmup_elapsed = performance_now() - warmup_started;
        report_event(
            reporter,
            &format!(
                "{{\"kind\":\"warmup-batch\",\"batchSize\":{batch_size},\"batchElapsedMs\":{elapsed:.6},\"elapsedMs\":{warmup_elapsed:.6}}}"
            ),
        )?;

        batch_size = calibrated_batch_size(
            batch_size,
            elapsed,
            config.target_sample_time_ms,
            MAX_BATCH_SIZE,
        );
    }
    batch_size = best_batch_size;
    report_event(
        reporter,
        &format!(
            "{{\"kind\":\"warmup-finished\",\"batchSize\":{batch_size},\"batchElapsedMs\":{best_batch_elapsed:.6},\"elapsedMs\":{warmup_elapsed:.6},\"iterations\":{warmup_iterations}}}"
        ),
    )?;

    let measurement_started = performance_now();
    let mut measurement_elapsed = 0.0;
    let mut samples_ns = Vec::with_capacity(config.sample_size as usize);
    let mut total_iterations = 0u64;
    while samples_ns.len() < config.sample_size as usize
        && measurement_elapsed < config.measurement_time_ms
    {
        let elapsed_ms = run_color_batch(
            source,
            target,
            mode,
            output,
            batch_size,
            config.row_band_height,
        );
        let sample_ns = elapsed_ms.max(0.0) * 1_000_000.0 / f64::from(batch_size);
        samples_ns.push(sample_ns);
        total_iterations += u64::from(batch_size);
        measurement_elapsed = performance_now() - measurement_started;
        if config.live_stats {
            report_event(
                reporter,
                &format!(
                    "{{\"kind\":\"measurement-progress\",\"samplesDone\":{},\"sampleSize\":{},\"elapsedMs\":{measurement_elapsed:.6},\"measurementTimeMs\":{:.6},\"totalIterations\":{total_iterations},\"batchSize\":{batch_size},\"samplesNs\":[{}]}}",
                    samples_ns.len(),
                    config.sample_size,
                    config.measurement_time_ms,
                    join_samples(&samples_ns)
                ),
            )?;
        }
    }

    Ok(WasmBenchmarkResult {
        batch_size,
        total_iterations,
        samples_ns,
    })
}

fn run_color_batch(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    mode: ColorBenchmarkMode,
    output: &mut [f32],
    batch_size: u32,
    row_band_height: usize,
) -> f64 {
    let started = performance_now();
    for _ in 0..batch_size {
        match mode {
            ColorBenchmarkMode::Scalar => {
                rgba8_to_color_space_f32_with_policy_into(source, target, false, output)
            }
            ColorBenchmarkMode::PooledDirect => {
                benchmark_color_pooled_direct(source, target, output, row_band_height)
            }
            ColorBenchmarkMode::PooledNoop => benchmark_color_pooled_noop(source, output),
            ColorBenchmarkMode::PooledCopy => benchmark_color_pooled_copy(source, output),
        }
        black_box(output.as_ref());
    }
    performance_now() - started
}

fn benchmark_color_pooled_direct(
    source: ImageView<'_, Rgba8>,
    target: ColorSpaceF32,
    output: &mut [f32],
    row_band_height: usize,
) {
    #[cfg(feature = "threads")]
    {
        crate::prod::color::rgba8_to_color_space_f32_parallel_with_band_height_into(
            source,
            target,
            output,
            row_band_height,
        );
    }

    #[cfg(not(feature = "threads"))]
    {
        black_box(row_band_height);
        rgba8_to_color_space_f32_with_policy_into(source, target, true, output);
    }
}

fn benchmark_color_pooled_noop(source: ImageView<'_, Rgba8>, output: &mut [f32]) {
    let dimensions = source.dimensions();
    let row_len = dimensions.width_usize() * Rgba8::CHANNEL_COUNT;
    assert_eq!(output.len(), dimensions.storage_len::<Rgba8>().unwrap());

    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        output.par_chunks_mut(row_len).for_each(|row| {
            black_box(row.as_ptr());
        });
    }

    #[cfg(not(feature = "threads"))]
    {
        for row in output.chunks_mut(row_len) {
            black_box(row.as_ptr());
        }
    }
}

fn benchmark_color_pooled_copy(source: ImageView<'_, Rgba8>, output: &mut [f32]) {
    let dimensions = source.dimensions();
    let row_len = dimensions.width_usize() * Rgba8::CHANNEL_COUNT;
    assert_eq!(output.len(), dimensions.storage_len::<Rgba8>().unwrap());

    #[cfg(feature = "threads")]
    {
        use rayon::prelude::*;
        output
            .par_chunks_mut(row_len)
            .enumerate()
            .for_each(|(y, output_row)| {
                let source_row = source.row(y as u32).expect("source row should exist");
                copy_color_row_for_benchmark(source_row, output_row);
            });
    }

    #[cfg(not(feature = "threads"))]
    {
        for (y, output_row) in output.chunks_mut(row_len).enumerate() {
            let source_row = source.row(y as u32).expect("source row should exist");
            copy_color_row_for_benchmark(source_row, output_row);
        }
    }
}

fn copy_color_row_for_benchmark(source_row: &[u8], output_row: &mut [f32]) {
    for (output, source) in output_row.iter_mut().zip(source_row) {
        *output = f32::from(*source);
    }
}

fn calibrated_batch_size(
    batch_size: u32,
    elapsed_ms: f64,
    target_ms: f64,
    max_batch_size: u32,
) -> u32 {
    if !elapsed_ms.is_finite() || elapsed_ms <= 0.0 {
        return batch_size.saturating_mul(2).clamp(1, max_batch_size);
    }
    let next = (f64::from(batch_size) * target_ms / elapsed_ms).round() as u32;
    next.clamp(1, max_batch_size)
}

fn run_resize_benchmark(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    resize: WasmResize,
    parallelization_policy: bool,
    output: &mut [u8],
    config: WasmBenchmarkConfig,
    reporter: Option<&Function>,
) -> Result<WasmBenchmarkResult, JsValue> {
    const MAX_BATCH_SIZE: u32 = 1 << 20;

    let mut batch_size = 1;
    let warmup_started = performance_now();
    let mut warmup_elapsed = 0.0;
    let mut warmup_batches = 0;
    let mut warmup_iterations = 0u64;
    let mut best_batch_size = batch_size;
    let mut best_batch_elapsed = f64::INFINITY;
    while warmup_elapsed < config.warm_up_time_ms
        || (config.warm_up_iterations > 0 && warmup_batches < config.warm_up_iterations)
    {
        let elapsed = run_resize_batch(
            source,
            output_dimensions,
            resize,
            parallelization_policy,
            output,
            batch_size,
            config.row_band_height,
        )?;
        if (elapsed - config.target_sample_time_ms).abs()
            < (best_batch_elapsed - config.target_sample_time_ms).abs()
        {
            best_batch_size = batch_size;
            best_batch_elapsed = elapsed;
        }
        warmup_batches += 1;
        warmup_iterations += u64::from(batch_size);
        warmup_elapsed = performance_now() - warmup_started;
        report_event(
            reporter,
            &format!(
                "{{\"kind\":\"warmup-batch\",\"batchSize\":{batch_size},\"batchElapsedMs\":{elapsed:.6},\"elapsedMs\":{warmup_elapsed:.6}}}"
            ),
        )?;

        batch_size = calibrated_batch_size(
            batch_size,
            elapsed,
            config.target_sample_time_ms,
            MAX_BATCH_SIZE,
        );
    }
    batch_size = best_batch_size;
    report_event(
        reporter,
        &format!(
            "{{\"kind\":\"warmup-finished\",\"batchSize\":{batch_size},\"batchElapsedMs\":{best_batch_elapsed:.6},\"elapsedMs\":{warmup_elapsed:.6},\"iterations\":{warmup_iterations}}}"
        ),
    )?;

    let measurement_started = performance_now();
    let mut measurement_elapsed = 0.0;
    let mut samples_ns = Vec::with_capacity(config.sample_size as usize);
    let mut total_iterations = 0u64;
    while samples_ns.len() < config.sample_size as usize
        && measurement_elapsed < config.measurement_time_ms
    {
        let elapsed_ms = run_resize_batch(
            source,
            output_dimensions,
            resize,
            parallelization_policy,
            output,
            batch_size,
            config.row_band_height,
        )?;
        let sample_ns = elapsed_ms.max(0.0) * 1_000_000.0 / f64::from(batch_size);
        samples_ns.push(sample_ns);
        total_iterations += u64::from(batch_size);
        measurement_elapsed = performance_now() - measurement_started;
        if config.live_stats {
            report_event(
                reporter,
                &format!(
                    "{{\"kind\":\"measurement-progress\",\"samplesDone\":{},\"sampleSize\":{},\"elapsedMs\":{measurement_elapsed:.6},\"measurementTimeMs\":{:.6},\"totalIterations\":{total_iterations},\"batchSize\":{batch_size},\"samplesNs\":[{}]}}",
                    samples_ns.len(),
                    config.sample_size,
                    config.measurement_time_ms,
                    join_samples(&samples_ns)
                ),
            )?;
        }
    }

    Ok(WasmBenchmarkResult {
        batch_size,
        total_iterations,
        samples_ns,
    })
}

fn run_resize_batch(
    source: ImageView<'_, Rgba8>,
    output_dimensions: ImageDimensions,
    resize: WasmResize,
    parallelization_policy: bool,
    output: &mut [u8],
    batch_size: u32,
    row_band_height: usize,
) -> Result<f64, JsValue> {
    let started = performance_now();
    for _ in 0..batch_size {
        if !(parallelization_policy
            && resize.run_pooled(source, output_dimensions, output, row_band_height)?)
        {
            resize.run(source, output_dimensions, output)?;
        }
        black_box(output.as_ref());
    }
    Ok(performance_now() - started)
}

fn positive_or_default(value: f64, default: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        default
    }
}

fn report_event(reporter: Option<&Function>, json: &str) -> Result<(), JsValue> {
    if let Some(reporter) = reporter {
        reporter.call1(&JsValue::NULL, &JsValue::from_str(json))?;
    }
    Ok(())
}

fn join_samples(samples_ns: &[f64]) -> String {
    samples_ns
        .iter()
        .map(|sample| format!("{sample:.0}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn benchmark_result_json(
    batch_size: u32,
    total_iterations: u64,
    output_byte_length: usize,
    checksum: u32,
    samples_ns: &[f64],
) -> String {
    let samples = join_samples(samples_ns);
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

fn checksum_f32(values: &[f32]) -> u32 {
    let mut hash = 2_166_136_261u32;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(16_777_619);
        }
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

fn parse_support_policy_and_execution(
    value: &str,
) -> Result<(SupportPolicy, ResizePlanScope, ResizeExecutionMode), JsValue> {
    match value {
        "" | "fixed" => Ok((
            SupportPolicy::Fixed,
            ResizePlanScope::FullImage,
            ResizeExecutionMode::PolicyDefault,
        )),
        "scale-aware" | "scale_aware" => Ok((
            SupportPolicy::ScaleAware,
            ResizePlanScope::FullImage,
            ResizeExecutionMode::PolicyDefault,
        )),
        "fixed+pooled-direct" => Ok((
            SupportPolicy::Fixed,
            ResizePlanScope::FullImage,
            ResizeExecutionMode::PooledDirect,
        )),
        "fixed+pooled-direct+per-band-plan" => Ok((
            SupportPolicy::Fixed,
            ResizePlanScope::PerBand,
            ResizeExecutionMode::PooledDirect,
        )),
        "fixed+pooled-noop" => Ok((
            SupportPolicy::Fixed,
            ResizePlanScope::FullImage,
            ResizeExecutionMode::PooledNoop,
        )),
        "fixed+pooled-copy" => Ok((
            SupportPolicy::Fixed,
            ResizePlanScope::FullImage,
            ResizeExecutionMode::PooledCopy,
        )),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_parallel_scale_policy_accepts_profitable_sweep_region() {
        assert!(nearest_parallel_scale_is_profitable(0.25, 0.25));
        assert!(nearest_parallel_scale_is_profitable(0.5, 0.5));
        assert!(nearest_parallel_scale_is_profitable(1.05, 1.05));
        assert!(nearest_parallel_scale_is_profitable(1.5, 1.5));
    }

    #[test]
    fn nearest_parallel_scale_policy_rejects_bad_sweep_region() {
        assert!(!nearest_parallel_scale_is_profitable(0.125, 0.125));
        assert!(!nearest_parallel_scale_is_profitable(0.99, 0.99));
        assert!(!nearest_parallel_scale_is_profitable(1.0, 1.0));
        assert!(!nearest_parallel_scale_is_profitable(0.5, 1.5));
    }

    #[test]
    fn nearest_exact_integer_upscale_detects_fast_copy_shapes() {
        assert!(nearest_exact_integer_upscale(
            ImageDimensions::new(800, 800).unwrap(),
            ImageDimensions::new(1600, 1600).unwrap(),
        ));
        assert!(nearest_exact_integer_upscale(
            ImageDimensions::new(2600, 4168).unwrap(),
            ImageDimensions::new(7800, 8336).unwrap(),
        ));
        assert!(!nearest_exact_integer_upscale(
            ImageDimensions::new(800, 800).unwrap(),
            ImageDimensions::new(1200, 1200).unwrap(),
        ));
    }

    #[test]
    fn nearest_production_tiling_policy_requires_threshold_and_policy_default() {
        let resize = WasmResize::parse("nearest", "center", "fixed").unwrap();
        assert_eq!(
            resize.production_tiling_policy(
                ImageDimensions::new(800, 800).unwrap(),
                ImageDimensions::new(200, 200).unwrap(),
            ),
            Some(NearestResizeTilingPolicy {
                row_band_height: NEAREST_ROW_BAND_HEIGHT,
            })
        );
        assert_eq!(
            resize.production_tiling_policy(
                ImageDimensions::new(800, 800).unwrap(),
                ImageDimensions::new(100, 100).unwrap(),
            ),
            None
        );

        assert_eq!(
            resize.production_tiling_policy(
                ImageDimensions::new(800, 800).unwrap(),
                ImageDimensions::new(1600, 1600).unwrap(),
            ),
            None
        );

        let diagnostic = WasmResize::parse("nearest", "center", "fixed+pooled-direct").unwrap();
        assert_eq!(
            diagnostic.production_tiling_policy(
                ImageDimensions::new(800, 800).unwrap(),
                ImageDimensions::new(200, 200).unwrap(),
            ),
            None
        );
    }
}
