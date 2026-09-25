//! Every allocation an effects call makes either succeeds or reports `wasm-memory-unavailable`.
//! Failing each one in turn must never abort the process.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::{error::ErrorCode, failure::Failure, request::WorkingSpace},
        effects::{decode_effects, EffectContext},
        pipeline::{
            effects::EffectsRequest,
            processor::{Boundary, Processor},
            quantize::InputBoundary,
        },
    },
};
use serde_json::json;

struct Injecting;
thread_local! {
    static FAIL_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: Injecting = Injecting;
unsafe impl GlobalAlloc for Injecting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|count| count.set(count.get() + 1));
        let fail = FAIL_AFTER.with(|remaining| match remaining.get() {
            Some(0) => {
                remaining.set(None);
                true
            }
            Some(value) => {
                remaining.set(Some(value - 1));
                false
            }
            None => false,
        });
        if fail {
            std::ptr::null_mut()
        } else {
            unsafe { System.alloc(layout) }
        }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

const WIDTH: u32 = 16;
const HEIGHT: u32 = 16;
const PALETTE: [PaletteEntry; 3] = [
    PaletteEntry::Color { rgb: [0, 0, 0] },
    PaletteEntry::Color { rgb: [237, 28, 36] },
    PaletteEntry::Color { rgb: [40, 80, 158] },
];

struct Io<'a>(&'a [u8]);

impl InputBoundary for Io<'_> {
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.0.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(self.0);
        Ok(())
    }
}

/// Result construction belongs to the adapter; this boundary only reports success.
impl Boundary for Io<'_> {
    type Output = ();
    fn complete(&mut self, _: &[u8], _: ImageDimensions) -> Result<(), Failure> {
        Ok(())
    }
}

/// Runs `call` once to count its allocations, then fails each one in turn on a fresh processor.
fn every_allocation_fails_cleanly(call: impl Fn(&mut Processor) -> Result<(), Failure>) {
    let mut processor = Processor::new(1 << 30, 0).unwrap();
    COUNT.with(|count| count.set(0));
    call(&mut processor).unwrap();
    let total = COUNT.with(Cell::get);
    assert!(total > 0);
    for index in 0..total {
        let mut processor = Processor::new(1 << 30, 0).unwrap();
        FAIL_AFTER.with(|remaining| remaining.set(Some(index)));
        let result = call(&mut processor);
        FAIL_AFTER.with(|remaining| remaining.set(None));
        if let Err(failure) = result {
            assert_eq!(
                failure.code,
                ErrorCode::WasmMemoryUnavailable,
                "allocation {index}"
            );
        }
        // The instance stays usable after the failure.
        call(&mut processor).unwrap();
    }
}

#[test]
fn effect_calls_report_every_failed_allocation() {
    let data: Vec<u8> = (0..WIDTH * HEIGHT)
        .flat_map(|i| [(i * 7) as u8, (i * 3) as u8, (255 - i) as u8, 255])
        .collect();
    let chain = decode_effects(
        &json!([
            { "effect": "curves", "enabled": true, "channel": "rgb", "points": [[0, 0], [0.4, 0.5], [1, 1]] },
            { "effect": "recolour", "enabled": true, "strength": 0.8, "recipe": null },
            { "effect": "hue-saturation", "enabled": true, "hue": 20, "saturation": 0.1, "lightness": 0 },
            { "effect": "levels", "enabled": true, "channel": "blue", "input": { "black": 0, "white": 1 },
              "gamma": 1.3, "output": { "black": 0, "white": 1 } },
        ])
        .to_string(),
    )
    .unwrap();
    let context = EffectContext {
        palette: &PALETTE,
        space: Some(WorkingSpace::Oklab),
        analyses: None,
    };
    let request = |effects| EffectsRequest {
        source_width: WIDTH,
        source_height: HEIGHT,
        effects,
        context,
    };
    every_allocation_fails_cleanly(|processor| {
        processor.apply_effects(request(&chain), &mut Io(&data))
    });
    every_allocation_fails_cleanly(|processor| {
        processor
            .analyze_recolour(request(&chain[..1]), &mut Io(&data))
            .map(drop)
    });
}
