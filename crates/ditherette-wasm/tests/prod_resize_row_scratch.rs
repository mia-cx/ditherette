use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::error::ErrorCode,
        resize::{
            common::allocation::CapacityBudget,
            scalar::{area, bilinear},
        },
        tiling::RowBandPlan,
    },
};

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

struct AllocationCounter;
thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: AllocationCounter = AllocationCounter;
unsafe impl GlobalAlloc for AllocationCounter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|count| count.set(count.get() + 1));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
    }
}

#[test]
fn caller_scratch_area_bands_keep_exact_scalar_bytes_and_reject_before_writes() {
    for (sw, sh, ow, oh) in [
        (14, 10, 7, 5),
        (7, 5, 14, 10),
        (13, 11, 7, 5),
        (13, 11, 13, 7),
        (13, 11, 7, 11),
        (7, 5, 7, 5),
    ] {
        let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
        let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
        let bytes: Vec<u8> = (0..sw * sh * 4).map(|i| (i * 71 + i / 11) as u8).collect();
        let source = ImageView::<Rgba8>::packed(&bytes, source_dimensions).unwrap();
        let plan = area::AreaResizePlan::new(source_dimensions, output_dimensions);
        let mut expected = vec![0; (ow * oh * 4) as usize];
        area::resize_area_rgba8_with_plan_into(
            source,
            ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
            &plan,
        );
        for band_height in [1, 2, 4, oh] {
            let bands = RowBandPlan::for_output_height(output_dimensions, band_height).unwrap();
            let required = plan.scratch_elements();
            let mut budget = CapacityBudget::new(required as u64 * 4);
            let mut scratch = budget.vector::<f32>(required).unwrap();
            scratch.resize(required, 0.0);
            let mut actual = vec![0; expected.len()];
            for band in bands.bands() {
                let dimensions = ImageDimensions::new(ow, band.height()).unwrap();
                let start = (band.y_start() * ow * 4) as usize;
                let end = (band.y_end() * ow * 4) as usize;
                if required > 0 {
                    actual[start..end].fill(219);
                    let error = area::resize_area_rgba8_rows_with_plan_and_scratch_into(
                        source,
                        ImageViewMut::packed(&mut actual[start..end], dimensions).unwrap(),
                        &plan,
                        band.y_start(),
                        &mut scratch[..required - 1],
                    )
                    .unwrap_err();
                    assert_eq!(error.code, ErrorCode::MemoryLimit);
                    assert!(actual[start..end].iter().all(|&byte| byte == 219));
                }
                let allocations = ALLOCATIONS.with(Cell::get);
                area::resize_area_rgba8_rows_with_plan_and_scratch_into(
                    source,
                    ImageViewMut::packed(&mut actual[start..end], dimensions).unwrap(),
                    &plan,
                    band.y_start(),
                    &mut scratch,
                )
                .unwrap();
                assert_eq!(ALLOCATIONS.with(Cell::get), allocations);
            }
            assert_eq!(
                actual, expected,
                "{sw}x{sh} -> {ow}x{oh}, band {band_height}"
            );
        }
    }
}

#[test]
fn caller_scratch_bilinear_bands_preserve_anchors_and_identity_axes() {
    use bilinear::alignment::ResizeAnchor;
    for (sw, sh, ow, oh) in [
        (13, 11, 7, 5),
        (7, 5, 13, 11),
        (13, 11, 13, 7),
        (13, 11, 7, 11),
        (7, 5, 7, 5),
    ] {
        let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
        let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
        let bytes: Vec<u8> = (0..sw * sh * 4).map(|i| (i * 71 + i / 11) as u8).collect();
        let source = ImageView::<Rgba8>::packed(&bytes, source_dimensions).unwrap();
        for anchor in [
            ResizeAnchor::TopLeft,
            ResizeAnchor::Center,
            ResizeAnchor::BottomRight,
        ] {
            let plan =
                bilinear::BilinearResizePlan::new(source_dimensions, output_dimensions, anchor);
            let mut expected = vec![0; (ow * oh * 4) as usize];
            bilinear::resize_bilinear_rgba8_with_plan_into(
                source,
                ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
                &plan,
            );
            let required = plan.scratch_elements();
            let mut budget = CapacityBudget::new(required as u64 * 4);
            let mut scratch = budget.vector::<f32>(required).unwrap();
            scratch.resize(required, 0.0);
            let mut actual = vec![219; expected.len()];
            for band in RowBandPlan::for_output_height(output_dimensions, 2)
                .unwrap()
                .bands()
            {
                let dimensions = ImageDimensions::new(ow, band.height()).unwrap();
                let start = (band.y_start() * ow * 4) as usize;
                let end = (band.y_end() * ow * 4) as usize;
                if required > 0 {
                    let error = bilinear::resize_bilinear_rgba8_rows_with_plan_and_scratch_into(
                        source,
                        ImageViewMut::packed(&mut actual[start..end], dimensions).unwrap(),
                        &plan,
                        band.y_start(),
                        &mut scratch[..required - 1],
                    )
                    .unwrap_err();
                    assert_eq!(error.code, ErrorCode::MemoryLimit);
                    assert!(actual[start..end].iter().all(|&byte| byte == 219));
                }
                let allocations = ALLOCATIONS.with(Cell::get);
                bilinear::resize_bilinear_rgba8_rows_with_plan_and_scratch_into(
                    source,
                    ImageViewMut::packed(&mut actual[start..end], dimensions).unwrap(),
                    &plan,
                    band.y_start(),
                    &mut scratch,
                )
                .unwrap();
                assert_eq!(ALLOCATIONS.with(Cell::get), allocations);
            }
            assert_eq!(actual, expected, "{sw}x{sh} -> {ow}x{oh}, {anchor:?}");
        }
    }
}
