//! Optional async package bridge. The module factory owns one verified backend.
//! CPU fallback stays in the existing scalar package processor.

use std::cell::RefCell;

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wgpu::web_sys::ImageBitmap;

use crate::{
    image::ImageDimensions,
    prod::{
        effects::decode_effects,
        gpu::{Backend, ChannelLookup, Fallback, LookupMemory},
    },
};

use super::processor::copy_input;

thread_local! {
    static BACKEND: RefCell<Option<Backend>> = const { RefCell::new(None) };
}

/// Idle bridge ownership deducted from the CPU fallback's available capacity.
#[wasm_bindgen(js_name = privateGpuEffectsOverhead)]
pub fn private_gpu_effects_overhead() -> f64 {
    LookupMemory::BRIDGE_BYTES as f64
}

#[wasm_bindgen(module = "/src/wasm/copy_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = completeResult)]
    fn complete_result(
        source: &[u8],
        width: u32,
        height: u32,
        sink: &JsValue,
    ) -> Result<(), JsValue>;
}

fn plan(
    effects: &str,
    width: u32,
    height: u32,
) -> Result<(Option<ChannelLookup>, LookupMemory), JsError> {
    let dimensions =
        ImageDimensions::new(width, height).map_err(|error| JsError::new(&error.to_string()))?;
    let steps = decode_effects(effects).map_err(|error| JsError::new(&error.to_string()))?;
    let stage =
        ChannelLookup::from_effects(&steps).map_err(|error| JsError::new(&error.to_string()))?;
    let mut memory = LookupMemory::new(dimensions);
    // wasm-bindgen's input string plus decoding's temporary JSON text capacity.
    memory.required_bytes += 2 * effects.len() as u64;
    Ok((stage, memory))
}

/// Zero means an unsupported chain; otherwise the full owned capacity required.
/// This call precedes the host snapshot and all image allocations.
#[wasm_bindgen(js_name = privateGpuEffectsPlan)]
pub fn private_gpu_effects_plan(effects: &str, width: u32, height: u32) -> Result<f64, JsError> {
    let (stage, memory) = plan(effects, width, height)?;
    Ok(if stage.is_some() {
        memory.required_bytes as f64
    } else {
        0.0
    })
}

/// Detects once per retained backend. The package serializes detection and processing.
#[wasm_bindgen(js_name = privateGpuEffectsDetect)]
pub async fn private_gpu_effects_detect(memory_limit: f64) -> String {
    let (_, dimensions, _) = ChannelLookup::known_answer();
    if LookupMemory::new(dimensions)
        .check(memory_limit as u64)
        .is_err()
    {
        return format!("{:?}", Fallback::MemoryLimit);
    }
    let backend = Backend::detect().await;
    let reason = match &backend {
        Backend::Cpu(reason) => format!("{reason:?}"),
        Backend::Gpu(_) => String::new(),
    };
    BACKEND.with(|slot| *slot.borrow_mut() = Some(backend));
    reason
}

/// Adapter/backend metadata from the verified retained device.
#[wasm_bindgen(js_name = privateGpuEffectsAdapter)]
pub fn private_gpu_effects_adapter() -> String {
    BACKEND.with(|slot| match slot.borrow().as_ref() {
        Some(Backend::Gpu(gpu)) => {
            let info = gpu.info();
            if info.name.is_empty() {
                // WebGPU may redact the description. Keep the backend/type honest.
                format!("{:?} ({:?})", info.backend, info.device_type)
            } else {
                format!("{} ({:?}, {:?})", info.name, info.backend, info.device_type)
            }
        }
        _ => String::new(),
    })
}

/// Returns an empty reason after writing durable JS-owned RGBA8 to the result sink.
/// Any reason asks the package to release this backend and run the frozen CPU path.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = privateGpuApplyEffects)]
pub async fn private_gpu_apply_effects(
    input: Uint8Array,
    width: u32,
    height: u32,
    effects: String,
    bitmap: Option<ImageBitmap>,
    memory_limit: f64,
    sink: JsValue,
) -> Result<String, JsError> {
    let (stage, memory) = plan(&effects, width, height)?;
    let Some(stage) = stage else {
        return Ok("UnsupportedChain".into());
    };
    if let Err(reason) = memory.check(memory_limit as u64) {
        return Ok(format!("{reason:?}"));
    }
    if u64::from(input.length()) != memory.rgba_bytes {
        return Err(JsError::new("source data no longer matches its dimensions"));
    }
    if bitmap
        .as_ref()
        .is_some_and(|bitmap| bitmap.width() != width || bitmap.height() != height)
    {
        return Err(JsError::new("bitmap dimensions must match the source"));
    }
    let dimensions =
        ImageDimensions::new(width, height).map_err(|error| JsError::new(&error.to_string()))?;
    let mut backend = BACKEND.with(|slot| slot.borrow_mut().take());
    let result = match backend.as_mut() {
        Some(backend) => match bitmap {
            Some(bitmap) => match backend {
                Backend::Gpu(gpu) => gpu.process_bitmap(&stage, &bitmap).await,
                Backend::Cpu(reason) => Err(reason.clone()),
            },
            None => {
                let mut rgba = Vec::new();
                if rgba.try_reserve_exact(memory.rgba_bytes as usize).is_err() {
                    Err(Fallback::Failed("source allocation failed".into()))
                } else {
                    rgba.resize(memory.rgba_bytes as usize, 0);
                    match copy_input(&mut rgba, &input) {
                        Ok(()) => {
                            backend
                                .lookup(&stage, dimensions, &rgba, memory_limit as u64)
                                .await
                        }
                        Err(_) => Err(Fallback::Failed("source copy failed".into())),
                    }
                }
            }
        },
        None => Err(Fallback::NoAdapter),
    };
    if let Some(backend) = backend.as_mut() {
        backend.retire_failure(&result);
    }
    BACKEND.with(|slot| *slot.borrow_mut() = backend);
    match result {
        Ok(rgba) => {
            complete_result(&rgba, width, height, &sink)
                .map_err(|_| JsError::new("result copy failed"))?;
            Ok(String::new())
        }
        Err(reason) => Ok(format!("{reason:?}")),
    }
}

/// Releases the device and pipeline before the host allocates CPU fallback resources.
#[wasm_bindgen(js_name = privateGpuEffectsDispose)]
pub fn private_gpu_effects_dispose() {
    BACKEND.with(|slot| *slot.borrow_mut() = None);
}
