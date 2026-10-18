//! Numeric resize admission and owned asynchronous image transport.

use super::{
    gpu_effects::BACKEND,
    processor::{copy_input, parse_resize, JsBoundary},
};
use crate::{
    image::ImageDimensions,
    prod::{
        contract::request::{
            Output, ResizeRequest, Source, MAX_OUTPUT_SIDE, MAX_PIXELS, MAX_SOURCE_SIDE,
        },
        gpu::{Fallback, GpuTiling, ResizeMemory},
        pipeline::processor::Boundary,
    },
};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/wasm/gpu_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch,js_name=checkJob)]
    fn check_job(sink: &JsValue, completed: u32) -> Result<bool, JsValue>;
}

fn tiling(rows: u32) -> GpuTiling {
    match rows {
        0 => GpuTiling::Auto,
        u32::MAX => GpuTiling::Disabled,
        rows => GpuTiling::Rows(rows),
    }
}

/// No source or string boundary is entered until all transient storage fits.
#[wasm_bindgen(js_name=privateGpuResizePlan)]
pub fn private_gpu_resize_plan(
    width: u32,
    height: u32,
    out_width: u32,
    out_height: u32,
    algorithm: f64,
    anchor: f64,
    support: f64,
    rows: u32,
) -> Result<f64, JsError> {
    if width == 0
        || height == 0
        || width > MAX_SOURCE_SIDE
        || height > MAX_SOURCE_SIDE
        || out_width == 0
        || out_height == 0
        || out_width > MAX_OUTPUT_SIDE
        || out_height > MAX_OUTPUT_SIDE
        || u64::from(width) * u64::from(height) > MAX_PIXELS
        || u64::from(out_width) * u64::from(out_height) > MAX_PIXELS
        || (rows != u32::MAX && rows > 32768)
    {
        return Err(JsError::new("invalid resize dimensions or tile height"));
    }
    let source = ImageDimensions::new(width, height).map_err(|e| JsError::new(&e.to_string()))?;
    let output =
        ImageDimensions::new(out_width, out_height).map_err(|e| JsError::new(&e.to_string()))?;
    let policy =
        parse_resize(algorithm, anchor, support).map_err(|e| JsError::new(&format!("{e:?}")))?;
    Ok(ResizeMemory::new(source, output, policy, tiling(rows)).required_bytes as f64)
}

#[wasm_bindgen(js_name=privateGpuResize)]
pub async fn private_gpu_resize(
    input: Uint8Array,
    width: u32,
    height: u32,
    out_width: u32,
    out_height: u32,
    algorithm: f64,
    anchor: f64,
    support: f64,
    rows: u32,
    coverage: bool,
    memory_limit: f64,
    sink: JsValue,
) -> Result<String, JsError> {
    if private_gpu_resize_plan(
        width, height, out_width, out_height, algorithm, anchor, support, rows,
    )? > memory_limit
    {
        return Ok("MemoryLimit".into());
    }
    let policy =
        parse_resize(algorithm, anchor, support).map_err(|e| JsError::new(&format!("{e:?}")))?;
    let bytes = u64::from(width) * u64::from(height) * 4;
    if u64::from(input.length()) != bytes {
        return Err(JsError::new("source dimensions no longer match"));
    }
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(bytes as usize)
        .map_err(|_| JsError::new("source allocation failed"))?;
    rgba.resize(bytes as usize, 0);
    copy_input(&mut rgba, &input).map_err(|_| JsError::new("source copy failed"))?;
    let mut backend = BACKEND.with(|slot| slot.borrow_mut().take());
    struct Control<'a> {
        sink: &'a JsValue,
        width: u32,
    }
    impl crate::prod::gpu::JobControl for Control<'_> {
        fn check(&mut self) -> Result<(), Fallback> {
            self.row(0)
        }
        fn row(&mut self, completed: u32) -> Result<(), Fallback> {
            match check_job(self.sink, completed * self.width) {
                Ok(true) => Ok(()),
                Ok(false) => Err(Fallback::Cancelled),
                Err(_) => Err(Fallback::Callback),
            }
        }
    }
    let mut control = Control {
        sink: &sink,
        width: out_width,
    };
    let result = match backend.as_mut() {
        Some(backend) => {
            backend
                .resize_checked(
                    ResizeRequest {
                        version: 1,
                        source: Source {
                            width,
                            height,
                            data: &rgba,
                        },
                        output: Output {
                            width: out_width,
                            height: out_height,
                            resize: policy,
                        },
                    },
                    coverage,
                    tiling(rows),
                    memory_limit as u64,
                    &mut control,
                )
                .await
        }
        None => Err(Fallback::NoAdapter),
    };
    BACKEND.with(|slot| *slot.borrow_mut() = backend);
    match result {
        Ok(bytes) => {
            crate::prod::gpu::JobControl::check(&mut control)
                .map_err(|e| JsError::new(&format!("{e:?}")))?;
            JsBoundary::new(&input, &sink)
                .and_then(|mut boundary| {
                    boundary.complete(
                        &bytes,
                        ImageDimensions::new(out_width, out_height).expect("validated output"),
                    )
                })
                .map_err(|_| JsError::new("resize result copy failed"))?;
            Ok(String::new())
        }
        Err(reason) => Ok(format!("{reason:?}")),
    }
}
