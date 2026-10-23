//! Optional async package bridge. The module factory owns one verified backend.
//! CPU fallback stays in the existing scalar package processor.

use std::cell::{Cell, RefCell};

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use crate::{
    image::ImageDimensions,
    prod::{
        effects::{chain::validate_chain, decode_effects, EffectContext, EffectStep},
        gpu::{Backend, ChannelLookup, Fallback, LookupMemory, Pointwise},
    },
};

use super::processor::copy_input;

thread_local! {
    pub(super) static BACKEND: RefCell<Option<Backend>> = const { RefCell::new(None) };
    // Locked wgpu forgets its JS error/loss closures. A factory gets one device
    // attempt; the host must retire the artifact after releasing that device.
    static DETECTION_ATTEMPTED: Cell<bool> = const { Cell::new(false) };
}

/// Diagnostic bridge record capacity. CPU fallback retires the artifact itself;
/// this fixed value is not a charge for keeping its entire memory reachable.
#[wasm_bindgen(js_name = privateGpuEffectsOverhead)]
pub fn private_gpu_effects_overhead() -> f64 {
    LookupMemory::BRIDGE_BYTES as f64
}

/// Numeric-only admission, called before any effects text crosses the ABI.
#[wasm_bindgen(js_name = privateGpuEffectsPlanningBytes)]
pub fn private_gpu_effects_planning_bytes(effects_bytes: u32) -> f64 {
    LookupMemory::planning_bytes(u64::from(effects_bytes)) as f64
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

enum EffectPlan {
    Lookup(ChannelLookup),
    Pointwise(
        Vec<EffectStep>,
        Option<crate::prod::contract::request::WorkingSpace>,
    ),
}

fn plan(
    effects: &str,
    width: u32,
    height: u32,
    space: f64,
    rows: u32,
) -> Result<(Option<EffectPlan>, LookupMemory), JsError> {
    if rows != u32::MAX && rows > 32768 {
        return Err(JsError::new("invalid tile height"));
    }
    let dimensions =
        ImageDimensions::new(width, height).map_err(|error| JsError::new(&error.to_string()))?;
    let steps = decode_effects(effects).map_err(|error| JsError::new(&error.to_string()))?;
    let lookup =
        ChannelLookup::from_effects(&steps).map_err(|error| JsError::new(&error.to_string()))?;
    if lookup.is_none() && Pointwise::supported(&steps) {
        let space = super::fields::parse_space(space);
        validate_chain(
            &steps,
            &EffectContext {
                space,
                ..Default::default()
            },
        )
        .map_err(|error| JsError::new(&error.to_string()))?;
        let mut memory = Pointwise::memory(dimensions, effects.len() as u64);
        memory.required_bytes +=
            LookupMemory::with_tiling(dimensions, super::gpu_resize::tiling(rows)).required_bytes
                - LookupMemory::new(dimensions).required_bytes;
        return Ok((Some(EffectPlan::Pointwise(steps, space)), memory));
    }
    let mut memory = LookupMemory::with_tiling(dimensions, super::gpu_resize::tiling(rows));
    // Reserve the Value tree, serde buffers and decoded curves, not just text.
    memory.required_bytes += LookupMemory::DECODER_BYTES_PER_JSON_BYTE * effects.len() as u64;
    Ok((lookup.map(EffectPlan::Lookup), memory))
}

/// Zero means an unsupported chain; otherwise the full owned capacity required.
/// This call precedes the host snapshot and all image allocations.
#[wasm_bindgen(js_name = privateGpuEffectsPlan)]
pub fn private_gpu_effects_plan(
    effects: &str,
    width: u32,
    height: u32,
    space: f64,
    rows: u32,
) -> Result<f64, JsError> {
    let (stage, memory) = plan(effects, width, height, space, rows)?;
    Ok(if stage.is_some() {
        memory.required_bytes as f64
    } else {
        0.0
    })
}

/// Detects at most once per owning artifact, including after disposal or failure.
#[wasm_bindgen(js_name = privateGpuEffectsDetect)]
pub async fn private_gpu_effects_detect(memory_limit: f64) -> String {
    if DETECTION_ATTEMPTED.with(|attempted| attempted.replace(true)) {
        return "ArtifactRetired".into();
    }
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
#[wasm_bindgen(js_name = privateGpuApplyEffects)]
pub async fn private_gpu_apply_effects(
    input: Uint8Array,
    width: u32,
    height: u32,
    effects: String,
    memory_limit: f64,
    sink: JsValue,
    space: f64,
    rows: u32,
) -> Result<String, JsError> {
    let (stage, memory) = plan(&effects, width, height, space, rows)?;
    let Some(stage) = stage else {
        return Ok("UnsupportedChain".into());
    };
    if let Err(reason) = memory.check(memory_limit as u64) {
        return Ok(format!("{reason:?}"));
    }
    if u64::from(input.length()) != memory.rgba_bytes {
        return Err(JsError::new("source data no longer matches its dimensions"));
    }
    let dimensions =
        ImageDimensions::new(width, height).map_err(|error| JsError::new(&error.to_string()))?;
    let mut backend = BACKEND.with(|slot| slot.borrow_mut().take());
    let result = match backend.as_mut() {
        Some(backend) => {
            let mut rgba = Vec::new();
            if rgba.try_reserve_exact(memory.rgba_bytes as usize).is_err() {
                Err(Fallback::Failed("source allocation failed".into()))
            } else {
                rgba.resize(memory.rgba_bytes as usize, 0);
                match copy_input(&mut rgba, &input) {
                    Ok(()) => match stage {
                        EffectPlan::Lookup(stage) => {
                            backend
                                .lookup_tiled(
                                    &stage,
                                    dimensions,
                                    &rgba,
                                    super::gpu_resize::tiling(rows),
                                    memory_limit as u64,
                                )
                                .await
                        }
                        EffectPlan::Pointwise(steps, space) => match Pointwise::new(steps, space) {
                            Ok(stage) => {
                                backend
                                    .pointwise_tiled(
                                        &stage,
                                        dimensions,
                                        &rgba,
                                        super::gpu_resize::tiling(rows),
                                        memory_limit as u64,
                                    )
                                    .await
                            }
                            Err(reason) => Err(reason),
                        },
                    },
                    Err(_) => Err(Fallback::Failed("source copy failed".into())),
                }
            }
        }
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
