use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageBuf, ImageDimensions, ImageView, PaletteIndex8, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            request::{AlphaPolicy, MatchPolicy, QuantizeRequest, Source},
        },
        quantize::{quantize, PreparedQuantizer, QuantizeError},
    },
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    mem::size_of,
};

struct Allocator;
thread_local! {
    static FAIL_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.with(|count| count.set(count.get() + 1));
        let fail = FAIL_AFTER.with(|count| match count.get() {
            Some(0) => {
                count.set(None);
                true
            }
            Some(value) => {
                count.set(Some(value - 1));
                false
            }
            None => false,
        });
        if fail {
            std::ptr::null_mut()
        } else {
            System.alloc(layout)
        }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
    }
}

const ALPHA: AlphaPolicy = AlphaPolicy::Preserve {
    threshold: 127.9999999,
};
const MATCHING: MatchPolicy = MatchPolicy::SrgbEuclidean;
const SOURCE: [u8; 8] = [73, 73, 73, 255, 73, 73, 73, 0];
fn request(palette: &[PaletteEntry]) -> QuantizeRequest<'_> {
    QuantizeRequest {
        version: 1,
        source: Source {
            width: 2,
            height: 1,
            data: &SOURCE,
        },
        palette,
        alpha: ALPHA,
        matching: MATCHING,
    }
}
fn palette() -> Vec<PaletteEntry> {
    vec![PaletteEntry::Color { rgb: [73; 3] }; 257]
}
fn budget(palette: &[PaletteEntry]) -> u64 {
    PreparedQuantizer::required_capacity_bytes(palette, ALPHA, MATCHING).unwrap()
        + size_of::<ImageBuf<PaletteIndex8>>() as u64
        + 2
}

#[test]
fn exact_capacity_includes_records_coordinates_and_both_warning_strings() {
    let palette = palette();
    let capacity = PreparedQuantizer::required_capacity_bytes(&palette, ALPHA, MATCHING).unwrap();
    let prepared = PreparedQuantizer::try_new(&palette, ALPHA, MATCHING, capacity).unwrap();
    assert_eq!(prepared.capacity_bytes(), capacity);
    assert_eq!(prepared.palette().warnings.len(), 2);
    let result = quantize(request(&palette), budget(&palette)).unwrap();
    assert_eq!(result.indices.data(), [0, 0]);
    assert_eq!(result.palette.rgba.len(), 256 * 4);
    let before = COUNT.with(Cell::get);
    let failure = quantize(request(&palette), budget(&palette) - 1).unwrap_err();
    assert_eq!(COUNT.with(Cell::get), before);
    assert!(
        matches!(failure, QuantizeError::Preparation(error) if error.code == ErrorCode::MemoryLimit)
    );
}

#[test]
fn every_reservation_failure_returns_without_allocating_an_error_and_recovers() {
    let palette = palette();
    let budget = budget(&palette);
    // RGBA palette, visible entries, warnings Vec, two strings, matcher, output indices.
    for successful in 0..7 {
        let before = COUNT.with(Cell::get);
        FAIL_AFTER.with(|count| count.set(Some(successful)));
        let result = quantize(request(&palette), budget);
        FAIL_AFTER.with(|count| count.set(None));
        assert_eq!(COUNT.with(Cell::get) - before, successful + 1);
        assert!(
            matches!(result, Err(QuantizeError::Preparation(error)) if error.code == ErrorCode::WasmMemoryUnavailable)
        );
        assert_eq!(
            quantize(request(&palette), budget).unwrap().indices.data(),
            [0, 0]
        );
    }
}

#[test]
fn prepared_execution_never_allocates_and_prior_outputs_survive_preparation_drop() {
    let entries = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color { rgb: [73; 3] },
    ];
    let prepared = PreparedQuantizer::try_new(&entries, ALPHA, MATCHING, u64::MAX).unwrap();
    let source = ImageView::<Rgba8>::packed(&SOURCE, ImageDimensions::new(2, 1).unwrap()).unwrap();
    let mut output = [0; 2];
    let before = COUNT.with(Cell::get);
    prepared.quantize_into(source, &mut output);
    assert_eq!(COUNT.with(Cell::get), before);
    assert_eq!(output, [1, 0]);
    let first = quantize(request(&entries), budget(&entries)).unwrap();
    drop(prepared);
    let _later = quantize(request(&palette()), u64::MAX).unwrap();
    assert_eq!(first.indices.data(), [1, 0]);
    assert_eq!(SOURCE, [73, 73, 73, 255, 73, 73, 73, 0]);
}

#[test]
fn invalid_preparation_settings_fail_before_allocation() {
    for (entries, alpha, matching, code) in [
        (&[][..], ALPHA, MATCHING, ErrorCode::InvalidPalette),
        (
            &[PaletteEntry::Transparent {}][..],
            AlphaPolicy::Preserve {
                threshold: f64::NAN,
            },
            MATCHING,
            ErrorCode::InvalidSettings,
        ),
        (
            &[PaletteEntry::Transparent {}][..],
            ALPHA,
            MatchPolicy::CielabCiede2000,
            ErrorCode::UnsupportedOperation,
        ),
    ] {
        let before = COUNT.with(Cell::get);
        let result = PreparedQuantizer::try_new(entries, alpha, matching, u64::MAX);
        assert_eq!(COUNT.with(Cell::get), before);
        assert!(matches!(result, Err(error) if error.code == code));
    }
}
