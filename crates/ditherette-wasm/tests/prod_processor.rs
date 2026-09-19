use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{Anchor, Output, ResizePolicy},
        },
        pipeline::processor::{Boundary, NearestRequest, Processor},
    },
    spec::resize::{common::alignment::ResizeAnchor, scalar::nearest::resize_nearest_into},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct TestAllocator;
thread_local! {
    static ALLOCATION_FAILURE: Cell<Option<usize>> = const { Cell::new(None) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator;

unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        let fail = ALLOCATION_FAILURE.with(|remaining| match remaining.get() {
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
            System.alloc(layout)
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
    }
}

#[derive(Default)]
struct Io {
    input: Vec<u8>,
    copy_calls: usize,
    complete_calls: usize,
    fail_copy: bool,
    fail_complete: bool,
}

impl Boundary for Io {
    type Output = Vec<u8>;
    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.input.len())
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.copy_calls += 1;
        if self.fail_copy {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::SourceData,
            ));
        }
        destination.copy_from_slice(&self.input);
        Ok(())
    }
    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        self.complete_calls += 1;
        if self.fail_complete {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        Ok(bytes.to_vec())
    }
}

fn request() -> NearestRequest {
    NearestRequest {
        source_width: 2,
        source_height: 1,
        output: Output {
            width: 3,
            height: 2,
            resize: ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
        },
    }
}
fn io() -> Io {
    Io {
        input: vec![10, 20, 30, 0, 40, 50, 60, 255],
        ..Io::default()
    }
}
fn budget() -> u64 {
    Processor::bookkeeping_bytes(512) + 8 + 24 + (3 * std::mem::size_of::<usize>() + 2 * 4) as u64
}

#[test]
fn exact_capacity_budget_passes_and_one_under_preflights_before_allocation_or_copy() {
    let mut input = io();
    let original = input.input.clone();
    let mut exact = Processor::new(budget(), 512).unwrap();
    let result = exact.resize(request(), &mut input).unwrap();
    assert_eq!(
        result,
        [
            10, 20, 30, 0, 40, 50, 60, 255, 40, 50, 60, 255, 10, 20, 30, 0, 40, 50, 60, 255, 40,
            50, 60, 255
        ]
    );
    assert_eq!(exact.peak_capacity_bytes(), budget());
    assert_eq!(input.input, original);

    let mut too_small = Processor::new(budget() - 1, 512).unwrap();
    input.copy_calls = 0;
    input.complete_calls = 0;
    let before = ALLOCATIONS.with(Cell::get);
    let failure = too_small.resize(request(), &mut input).unwrap_err();
    assert_eq!(ALLOCATIONS.with(Cell::get), before);
    assert_eq!(
        failure,
        Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
    );
    assert_eq!((input.copy_calls, input.complete_calls), (0, 0));
}

#[test]
fn each_real_reservation_failure_reports_without_allocating_an_error_and_recovers() {
    // Two coordinate maps, then the owned source and output buffers.
    for successful_allocations in 0..4 {
        let mut processor = Processor::new(budget(), 512).unwrap();
        let mut input = io();
        let before = ALLOCATIONS.with(Cell::get);
        ALLOCATION_FAILURE.with(|remaining| remaining.set(Some(successful_allocations)));
        let failure = processor.resize(request(), &mut input).unwrap_err();
        ALLOCATION_FAILURE.with(|remaining| remaining.set(None));
        assert_eq!(
            ALLOCATIONS.with(Cell::get) - before,
            successful_allocations + 1
        );
        assert_eq!(
            failure,
            Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
        );
        assert_eq!((input.copy_calls, input.complete_calls), (0, 0));
        assert_eq!(processor.resize(request(), &mut input).unwrap().len(), 24);
    }
}

#[test]
fn near_identity_span_reservation_is_fallible_and_recovers_before_copy() {
    let request = NearestRequest {
        source_width: 21,
        source_height: 21,
        output: Output {
            width: 20,
            height: 20,
            resize: ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
        },
    };
    // The near-identity path additionally reserves the copy-span Vec.
    for successful_allocations in 0..5 {
        let mut processor = Processor::new(1_000_000, 512).unwrap();
        let mut input = Io {
            input: vec![73; 21 * 21 * 4],
            ..Io::default()
        };
        ALLOCATION_FAILURE.with(|remaining| remaining.set(Some(successful_allocations)));
        let failure = processor.resize(request, &mut input).unwrap_err();
        ALLOCATION_FAILURE.with(|remaining| remaining.set(None));
        assert_eq!(
            failure,
            Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
        );
        assert_eq!((input.copy_calls, input.complete_calls), (0, 0));
        assert_eq!(
            processor.resize(request, &mut input).unwrap(),
            vec![73; 20 * 20 * 4]
        );
    }
}

#[test]
fn input_and_complete_failures_drop_work_recover_and_preserve_prior_results() {
    let mut processor = Processor::new(budget(), 512).unwrap();
    let mut input = io();
    let first = processor.resize(request(), &mut input).unwrap();
    input.fail_copy = true;
    assert_eq!(
        processor.resize(request(), &mut input).unwrap_err().path,
        ErrorPath::SourceData
    );
    input.fail_copy = false;
    input.fail_complete = true;
    assert_eq!(
        processor.resize(request(), &mut input).unwrap_err().path,
        ErrorPath::Output
    );
    input.fail_complete = false;
    input.input.fill(71);
    let second = processor.resize(request(), &mut input).unwrap();
    assert!(second.iter().all(|&byte| byte == 71));
    assert_eq!(&first[..4], &[10, 20, 30, 0]);
    processor.dispose().unwrap();
    processor.dispose().unwrap();
    assert_eq!(
        processor.resize(request(), &mut input).unwrap_err().code,
        ErrorCode::Disposed
    );
    assert!(Processor::new(budget(), 512)
        .unwrap()
        .resize(request(), &mut input)
        .is_ok());
}

#[test]
fn invalid_settings_storage_and_tiny_initialization_are_allocation_free() {
    let mut processor = Processor::new(budget(), 512).unwrap();
    let mut input = io();
    for (request, expected) in [
        (
            NearestRequest {
                source_width: 0,
                ..request()
            },
            ErrorPath::SourceWidth,
        ),
        (
            NearestRequest {
                source_height: 32769,
                ..request()
            },
            ErrorPath::SourceHeight,
        ),
        (
            NearestRequest {
                output: Output {
                    width: 0,
                    ..request().output
                },
                ..request()
            },
            ErrorPath::OutputWidth,
        ),
        (
            NearestRequest {
                output: Output {
                    resize: ResizePolicy::Area {},
                    ..request().output
                },
                ..request()
            },
            ErrorPath::OutputResize,
        ),
    ] {
        let before = ALLOCATIONS.with(Cell::get);
        let failure = processor.resize(request, &mut input).unwrap_err();
        assert_eq!(ALLOCATIONS.with(Cell::get), before);
        assert_eq!(failure.path, expected);
    }
    input.input.pop();
    assert_eq!(
        processor.resize(request(), &mut input).unwrap_err().path,
        ErrorPath::SourceData
    );
    assert_eq!(input.copy_calls, 0);
    assert_eq!(
        Processor::new(1, 512).unwrap_err().code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(
        Processor::new(0, 512).unwrap_err().code,
        ErrorCode::InvalidSettings
    );
    assert_eq!(
        Processor::new(2147483649, 512).unwrap_err().code,
        ErrorCode::InvalidSettings
    );
}

#[test]
fn processor_bytes_match_frozen_nearest_across_all_anchors_and_shapes() {
    let anchors = [
        (Anchor::TopLeft, ResizeAnchor::TopLeft),
        (Anchor::Top, ResizeAnchor::Top),
        (Anchor::TopRight, ResizeAnchor::TopRight),
        (Anchor::Left, ResizeAnchor::Left),
        (Anchor::Center, ResizeAnchor::Center),
        (Anchor::Right, ResizeAnchor::Right),
        (Anchor::BottomLeft, ResizeAnchor::BottomLeft),
        (Anchor::Bottom, ResizeAnchor::Bottom),
        (Anchor::BottomRight, ResizeAnchor::BottomRight),
    ];
    for (sw, sh, ow, oh) in [
        (1, 1, 3, 5),
        (7, 1, 3, 1),
        (1, 7, 1, 3),
        (4, 4, 3, 3),
        (3, 5, 3, 5),
        (2, 3, 5, 2),
    ] {
        for (anchor, oracle_anchor) in anchors {
            let mut input = Io {
                input: (0..sw * sh * 4).map(|n| (n * 73 % 256) as u8).collect(),
                ..Io::default()
            };
            let source = ImageDimensions::new(sw, sh).unwrap();
            let output = ImageDimensions::new(ow, oh).unwrap();
            let mut expected = vec![0; (ow * oh * 4) as usize];
            resize_nearest_into(
                ImageView::<Rgba8>::packed(&input.input, source).unwrap(),
                ImageViewMut::packed(&mut expected, output).unwrap(),
                oracle_anchor,
            );
            let request = NearestRequest {
                source_width: sw,
                source_height: sh,
                output: Output {
                    width: ow,
                    height: oh,
                    resize: ResizePolicy::Nearest { anchor },
                },
            };
            let actual = Processor::new(1024 * 1024, 512)
                .unwrap()
                .resize(request, &mut input)
                .unwrap();
            assert_eq!(actual, expected);
        }
    }
}
