//! Private scalar ABI. Each package instance loads a separate Wasm module.
//!
//! Generated classes would allocate Rc after construction and hold mutable
//! borrows across JavaScript calls. Module state avoids both behaviors.

use std::{
    cell::{Cell, RefCell},
    mem::{replace, size_of},
};

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use crate::{
    image::ImageDimensions,
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{
                Anchor, Output, ResizePolicy, Support, MAX_MEMORY_LIMIT_BYTES, MAX_OUTPUT_SIDE,
                MAX_SOURCE_SIDE,
            },
        },
        pipeline::processor::{Boundary, Processor, ResizeRequest},
    },
};

#[wasm_bindgen(module = "/src/wasm/copy_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = inputLength)]
    pub(super) fn input_length(source: &Uint8Array) -> Result<f64, JsValue>;
    #[wasm_bindgen(catch, js_name = copyInput)]
    pub(super) fn copy_input(destination: &mut [u8], source: &Uint8Array) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = completeResult)]
    fn complete_result(
        source: &[u8],
        width: u32,
        height: u32,
        sink: &JsValue,
    ) -> Result<(), JsValue>;
}

enum Slot {
    Uninitialized,
    Ready(Processor),
    Busy,
    Disposed,
}

thread_local! {
    static INSTANCE: RefCell<Slot> = const { RefCell::new(Slot::Uninitialized) };
    static ERROR_PATH: Cell<ErrorPath> = const { Cell::new(ErrorPath::Instance) };
}

// Pinned wasm-bindgen 0.2.121 reserves 128 usize slots on its first owned handle.
// This stays fixed for the bounded borrowed-input/result/caught-error protocol.
const EXTERNREF_CAPACITY_BYTES: u64 = 128 * 4;
const BOUNDARY_CAPACITY_BYTES: u64 = EXTERNREF_CAPACITY_BYTES
    + size_of::<RefCell<Slot>>() as u64
    + size_of::<Cell<ErrorPath>>() as u64
    + size_of::<JsBoundary<'static>>() as u64;

/// Private fixture/accounting observation, not a public package method.
#[wasm_bindgen(js_name = privateMemoryOverhead)]
pub fn private_memory_overhead() -> u32 {
    Processor::bookkeeping_bytes(BOUNDARY_CAPACITY_BYTES) as u32
}

/// Last numeric failure path; the wrapper reads this immediately after failure.
#[wasm_bindgen(js_name = privateErrorPath)]
pub fn private_error_path() -> u32 {
    ERROR_PATH.with(|path| path.get() as u32)
}

/// Validates and preflights before priming boundary handles.
/// A trap in this function discards the isolated factory during package initialization.
#[wasm_bindgen(js_name = privateInitialize)]
pub fn private_initialize(memory_limit: f64) -> u32 {
    let available = INSTANCE.with(|instance| {
        let slot = instance.borrow();
        match &*slot {
            Slot::Uninitialized => Ok(()),
            Slot::Busy => Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance)),
            Slot::Disposed => Err(Failure::new(ErrorCode::Disposed, ErrorPath::Instance)),
            Slot::Ready(_) => Err(Failure::new(ErrorCode::Initialization, ErrorPath::Instance)),
        }
    });
    if let Err(error) = available {
        return status(error);
    }
    if !memory_limit.is_finite()
        || memory_limit.fract() != 0.0
        || !(1.0..=MAX_MEMORY_LIMIT_BYTES as f64).contains(&memory_limit)
    {
        return status(Failure::new(
            ErrorCode::InvalidSettings,
            ErrorPath::MemoryLimitBytes,
        ));
    }
    let processor = match Processor::new(memory_limit as u64, BOUNDARY_CAPACITY_BYTES) {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    INSTANCE.with(|instance| *instance.borrow_mut() = Slot::Busy);
    // Prime one numeric handle, then release it. No processor buffer exists yet.
    // A slab-allocation trap retires this factory at the package initialization boundary.
    // Processing imports return only scalars/void, avoiding caught owned-return leaks.
    let handle = JsValue::from_f64(memory_limit);
    drop(handle);
    INSTANCE.with(|instance| *instance.borrow_mut() = Slot::Ready(processor));
    0
}

/// A borrowed externref enters without a generated input byte allocation.
/// Success writes the complete result to a private plain sink. Every return is a numeric status.
#[wasm_bindgen(js_name = privateResize)]
pub fn private_resize(
    input: &Uint8Array,
    source_width: f64,
    source_height: f64,
    output_width: f64,
    output_height: f64,
    algorithm: f64,
    anchor: f64,
    support: f64,
    result_sink: &JsValue,
) -> u32 {
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        if matches!(algorithm, 0.0 | 1.0 | 2.0) && support != 0.0 {
            return Err(Failure::new(
                ErrorCode::InvalidSettings,
                ErrorPath::OutputResize,
            ));
        }
        if algorithm == 1.0 && anchor != 0.0 {
            return Err(Failure::new(
                ErrorCode::InvalidSettings,
                ErrorPath::OutputAnchor,
            ));
        }
        let request = ResizeRequest {
            source_width: dimension(
                source_width,
                MAX_SOURCE_SIDE,
                ErrorCode::InvalidImage,
                ErrorPath::SourceWidth,
            )?,
            source_height: dimension(
                source_height,
                MAX_SOURCE_SIDE,
                ErrorCode::InvalidImage,
                ErrorPath::SourceHeight,
            )?,
            output: Output {
                width: dimension(
                    output_width,
                    MAX_OUTPUT_SIDE,
                    ErrorCode::InvalidSettings,
                    ErrorPath::OutputWidth,
                )?,
                height: dimension(
                    output_height,
                    MAX_OUTPUT_SIDE,
                    ErrorCode::InvalidSettings,
                    ErrorPath::OutputHeight,
                )?,
                resize: match algorithm {
                    0.0 => ResizePolicy::Nearest {
                        anchor: parse_anchor(anchor)?,
                    },
                    1.0 => ResizePolicy::Area {},
                    2.0 => ResizePolicy::Bilinear {
                        anchor: parse_anchor(anchor)?,
                    },
                    3.0 => ResizePolicy::Bicubic {
                        anchor: parse_anchor(anchor)?,
                        support: parse_support(support)?,
                    },
                    4.0 => ResizePolicy::Lanczos2 {
                        anchor: parse_anchor(anchor)?,
                        support: parse_support(support)?,
                    },
                    5.0 => ResizePolicy::Lanczos3 {
                        anchor: parse_anchor(anchor)?,
                        support: parse_support(support)?,
                    },
                    _ => {
                        return Err(Failure::new(
                            ErrorCode::InvalidSettings,
                            ErrorPath::OutputResize,
                        ))
                    }
                },
            },
        };
        processor.resize(request, &mut JsBoundary { input, result_sink })
    })();
    // No RefCell borrow or generated &mut self borrow spans a JavaScript call.
    INSTANCE.with(|instance| *instance.borrow_mut() = Slot::Ready(processor));
    result.map_or_else(status, |_| 0)
}

/// Drops processor ownership idempotently. Fixed Wasm/glue capacity remains module-owned.
#[wasm_bindgen(js_name = privateDispose)]
pub fn private_dispose() -> u32 {
    let result = INSTANCE.with(|instance| {
        let mut slot = instance.borrow_mut();
        if matches!(&*slot, Slot::Busy) {
            return Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance));
        }
        if let Slot::Ready(processor) = &mut *slot {
            processor.dispose()?;
        }
        *slot = Slot::Disposed;
        Ok(())
    });
    result.map_or_else(status, |_| 0)
}

pub(super) fn take_ready() -> Result<Processor, Failure> {
    INSTANCE.with(|instance| {
        let mut slot = instance.borrow_mut();
        match &*slot {
            Slot::Uninitialized => {
                Err(Failure::new(ErrorCode::Initialization, ErrorPath::Instance))
            }
            Slot::Busy => Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance)),
            Slot::Disposed => Err(Failure::new(ErrorCode::Disposed, ErrorPath::Instance)),
            Slot::Ready(_) => {
                let Slot::Ready(processor) = replace(&mut *slot, Slot::Busy) else {
                    unreachable!()
                };
                Ok(processor)
            }
        }
    })
}

pub(super) struct JsBoundary<'a> {
    pub(super) input: &'a Uint8Array,
    pub(super) result_sink: &'a JsValue,
}

impl Boundary for JsBoundary<'_> {
    type Output = ();
    fn input_len(&mut self) -> Result<usize, Failure> {
        input_length(self.input)
            .map(|length| length as usize)
            .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData))
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        copy_input(destination, self.input)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::SourceData))
    }
    fn complete(&mut self, bytes: &[u8], dimensions: ImageDimensions) -> Result<(), Failure> {
        complete_result(
            bytes,
            dimensions.width(),
            dimensions.height(),
            self.result_sink,
        )
        .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Output))
    }
}

pub(super) fn restore_ready(processor: Processor) {
    INSTANCE.with(|instance| *instance.borrow_mut() = Slot::Ready(processor));
}

pub(super) fn status(error: Failure) -> u32 {
    ERROR_PATH.with(|path| path.set(error.path));
    error.status()
}

pub(super) fn dimension(
    value: f64,
    limit: u32,
    code: ErrorCode,
    path: ErrorPath,
) -> Result<u32, Failure> {
    if !value.is_finite() || value.fract() != 0.0 || !(1.0..=limit as f64).contains(&value) {
        return Err(Failure::new(code, path));
    }
    Ok(value as u32)
}

fn parse_anchor(value: f64) -> Result<Anchor, Failure> {
    match value {
        0.0 => Ok(Anchor::TopLeft),
        1.0 => Ok(Anchor::Top),
        2.0 => Ok(Anchor::TopRight),
        3.0 => Ok(Anchor::Left),
        4.0 => Ok(Anchor::Center),
        5.0 => Ok(Anchor::Right),
        6.0 => Ok(Anchor::BottomLeft),
        7.0 => Ok(Anchor::Bottom),
        8.0 => Ok(Anchor::BottomRight),
        _ => Err(Failure::new(
            ErrorCode::InvalidSettings,
            ErrorPath::OutputAnchor,
        )),
    }
}

fn parse_support(value: f64) -> Result<Support, Failure> {
    match value {
        0.0 => Ok(Support::Fixed),
        1.0 => Ok(Support::ScaleAware),
        _ => Err(Failure::new(
            ErrorCode::InvalidSettings,
            ErrorPath::OutputResize,
        )),
    }
}
