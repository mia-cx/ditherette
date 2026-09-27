//! Coverage allocations fail through the recoverable production error contract.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::{error::ErrorCode, failure::Failure, request::RecipeV1},
        pipeline::{
            process::ProcessRequest,
            processor::Processor,
            quantize::{IndexedMetadataRef, InputBoundary, QuantizeBoundary},
        },
    },
    spec,
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

const PALETTE: [PaletteEntry; 3] = [
    PaletteEntry::Transparent {},
    PaletteEntry::Color { rgb: [20, 40, 80] },
    PaletteEntry::Color {
        rgb: [220, 180, 90],
    },
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
impl QuantizeBoundary for Io<'_> {
    type Output = ();
    fn complete(
        &mut self,
        _: &[u8],
        _: ImageDimensions,
        _: IndexedMetadataRef<'_>,
    ) -> Result<(), Failure> {
        Ok(())
    }
}

fn source(opaque: bool) -> Vec<u8> {
    (0..16 * 12)
        .flat_map(|i| {
            [
                (i * 3) as u8,
                (i * 7) as u8,
                (255 - i) as u8,
                if opaque { 255 } else { (i * 11) as u8 },
            ]
        })
        .collect()
}

fn recipe(resize: serde_json::Value) -> RecipeV1 {
    let v2 = spec::effects::decode_recipe_v2(
        &json!({
            "version": 2,
            "effects": [],
            "output": { "width": 9, "height": 7, "resize": resize },
            "alpha": { "mode": "preserve", "threshold": 127.5 },
            "match": "oklab-euclidean",
            "dither": { "family": "none" }
        })
        .to_string(),
    )
    .unwrap();
    let mut terminal = serde_json::to_value(v2.terminal()).unwrap();
    terminal["version"] = json!(1);
    serde_json::from_value(terminal).unwrap()
}

fn run(processor: &mut Processor, data: &[u8], recipe: RecipeV1) -> Result<(), Failure> {
    processor.process_effects(
        ProcessRequest {
            source_width: 16,
            source_height: 12,
            palette: &PALETTE,
            recipe,
        },
        &[],
        &mut Io(data),
    )
}

fn every_allocation_fails_cleanly(data: &[u8], recipe: RecipeV1) {
    let mut processor = Processor::new(64 * 1024 * 1024, 0).unwrap();
    COUNT.with(|count| count.set(0));
    run(&mut processor, data, recipe).unwrap();
    let total = COUNT.with(Cell::get);
    assert!(total > 0);
    for index in 0..total {
        let mut processor = Processor::new(64 * 1024 * 1024, 0).unwrap();
        FAIL_AFTER.with(|remaining| remaining.set(Some(index)));
        let result = run(&mut processor, data, recipe);
        FAIL_AFTER.with(|remaining| remaining.set(None));
        if let Err(failure) = result {
            assert_eq!(
                failure.code,
                ErrorCode::WasmMemoryUnavailable,
                "allocation {index}"
            );
        }
        run(&mut processor, data, recipe).unwrap();
    }
}

#[test]
fn coverage_allocation_failures_leave_the_processor_reusable() {
    let data = source(false);
    for resize in [
        json!({ "algorithm": "area" }),
        json!({ "algorithm": "bilinear", "anchor": "center" }),
        json!({ "algorithm": "bicubic", "anchor": "center", "support": "scale-aware" }),
        json!({ "algorithm": "trilinear", "anchor": "center" }),
    ] {
        every_allocation_fails_cleanly(&data, recipe(resize));
    }
}

#[test]
fn nearest_and_opaque_bypasses_do_not_pay_for_coverage_carriers() {
    let translucent = source(false);
    let opaque = source(true);
    let filtered = recipe(json!({ "algorithm": "bilinear", "anchor": "center" }));
    let nearest = recipe(json!({ "algorithm": "nearest", "anchor": "center" }));
    let peak = |data: &[u8], recipe| {
        let mut processor = Processor::new(64 * 1024 * 1024, 0).unwrap();
        run(&mut processor, data, recipe).unwrap();
        processor.peak_capacity_bytes()
    };
    let covered = peak(&translucent, filtered);
    assert!(covered > peak(&opaque, filtered));
    assert!(covered > peak(&translucent, nearest));
}
