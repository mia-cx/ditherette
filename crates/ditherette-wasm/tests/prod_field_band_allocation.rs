use ditherette_wasm::{
    image::{contracts::PaletteEntry, ImageDimensions, ImageView, Rgba8},
    prod::{
        color::packed::Converter,
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{AlphaPolicy, MatchPolicy, Placement, WorkingSpace},
        },
        dither::{perturb, random_noise},
        quantize::PreparedQuantizer,
        tiling::{RowBandBuffers, WorkerBudget},
    },
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    mem::size_of,
};

struct TestAllocator;
thread_local! {
    static FAIL_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator;
unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        let fail = FAIL_AFTER.with(|remaining| match remaining.get() {
            Some(0) => {
                remaining.set(None);
                true
            }
            Some(n) => {
                remaining.set(Some(n - 1));
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

#[test]
fn field_workers_charge_live_converters_before_reservations_and_recover_after_each_failure() {
    let dimensions = ImageDimensions::new(5, 11).unwrap();
    let pool = WorkerBudget::new(4);
    let bytes: Vec<_> = (0..220).map(|n| (n * 73) as u8).collect();
    let source = ImageView::<Rgba8>::packed(&bytes, dimensions).unwrap();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
    ];
    let prepared = PreparedQuantizer::try_new(
        &palette,
        AlphaPolicy::Premultiplied {},
        MatchPolicy::OklabEuclidean,
        u64::MAX,
    )
    .unwrap();
    for (height, active) in [(1, 4), (3, 4), (7, 2), (20, 1)] {
        let metadata =
            RowBandBuffers::<()>::required_bytes(dimensions, height, pool, 4, &|_| Ok(0)).unwrap();
        let required = perturb::required_band_capacity_bytes(dimensions, height, pool, 4).unwrap();
        assert_eq!(required, metadata + active * size_of::<Converter>() as u64);
        // This is the enclosing separable call's independent accounting, not a new allocation plane.
        let caller_owned =
            prepared.capacity_bytes() + (220 + 220 + 55) + 3 * size_of::<Vec<u8>>() as u64;
        let complete_limit = caller_owned + required;
        let before = ALLOCATIONS.with(Cell::get);
        let error = perturb::try_band_buffers(
            dimensions,
            height,
            pool,
            4,
            complete_limit - caller_owned - 1,
        )
        .err()
        .unwrap();
        assert_eq!(
            (error.code, error.path),
            (ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
        );
        assert_eq!(ALLOCATIONS.with(Cell::get), before);
        let before = ALLOCATIONS.with(Cell::get);
        let mut work = perturb::try_band_buffers(dimensions, height, pool, 4, required).unwrap();
        let reservations = ALLOCATIONS.with(Cell::get) - before;
        assert_eq!(work.work().active_workers(), active as u32);
        assert_eq!(
            work.capacity_bytes() + perturb::band_working_capacity_bytes(active as u32),
            required
        );
        for reservation in 0..reservations {
            FAIL_AFTER.with(|remaining| remaining.set(Some(reservation)));
            let result = perturb::try_band_buffers(dimensions, height, pool, 4, required);
            FAIL_AFTER.with(|remaining| remaining.set(None));
            let error = result.err().unwrap();
            assert_eq!(
                (error.code, error.path),
                (ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
            );
        }
        let mut output = vec![213; 220];
        let mut indices = vec![213; 55];
        let run = |work: &mut RowBandBuffers<()>,
                   output: &mut [u8],
                   progress: &mut dyn FnMut(u64) -> Result<(), Failure>| {
            perturb::perturb_by_field_bands_into(
                source,
                output,
                WorkingSpace::Oklab,
                0.7,
                Placement::Adaptive {
                    radius: 4,
                    threshold: 0.05,
                    softness: 0.025,
                },
                work,
                |_, _, index| random_noise::random_noise_at(123, index),
                &mut |rows| progress(rows),
            )
        };
        let caller = std::thread::current().id();
        let mut callbacks = 0;
        let error = run(&mut work, &mut output, &mut |_| {
            assert_eq!(std::thread::current().id(), caller);
            callbacks += 1;
            Err(Failure::new(ErrorCode::Callback, ErrorPath::OnProgress))
        })
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::Callback);
        assert_eq!(callbacks, 1);
        for assignment in work.work().assignments() {
            for band in &assignment.bands()[1..] {
                assert!(
                    output[(band.y_start() * 20) as usize..(band.y_end() * 20) as usize]
                        .iter()
                        .all(|&byte| byte == 213)
                );
            }
        }
        run(&mut work, &mut output, &mut |_| Ok(())).unwrap();
        let perturbed = ImageView::packed(&output, dimensions).unwrap();
        prepared
            .quantize_bands_into(perturbed, &mut indices, &mut work, &mut |_| Ok(()))
            .unwrap();
        let mut expected = vec![0; 55];
        prepared.quantize_into(perturbed, &mut expected);
        assert_eq!(indices, expected);
        // Quantization adds no per-worker converter or palette; it reuses the same metadata owner.
        assert_eq!(work.capacity_bytes(), metadata);
    }
}
