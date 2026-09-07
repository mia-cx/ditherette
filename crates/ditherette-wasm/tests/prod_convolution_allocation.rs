use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{error::ErrorCode, failure::Failure},
        resize::{
            common::allocation::CapacityBudget,
            scalar::{
                bicubic::{
                    resize_bicubic_rgba8_into, resize_bicubic_rgba8_with_plan_and_scratch_into,
                    BicubicResizePlan,
                },
                convolution::{ResizeAnchor, SupportPolicy},
                lanczos::{
                    resize_lanczos2_rgba8_into, resize_lanczos3_rgba8_into,
                    resize_lanczos_rgba8_with_plan_and_scratch_into, LanczosResizePlan,
                },
            },
        },
    },
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    num::NonZeroU32,
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

enum Plan {
    Bicubic(BicubicResizePlan),
    Lanczos(LanczosResizePlan),
}
impl Plan {
    fn required(
        mode: u32,
        source: ImageDimensions,
        output: ImageDimensions,
        policy: SupportPolicy,
    ) -> Result<u64, Failure> {
        match mode {
            0 => BicubicResizePlan::required_bytes(source, output, policy),
            radius => LanczosResizePlan::required_bytes(
                source,
                output,
                NonZeroU32::new(radius).unwrap(),
                policy,
            ),
        }
    }
    fn new(
        mode: u32,
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: ResizeAnchor,
        policy: SupportPolicy,
        budget: &mut CapacityBudget,
    ) -> Result<Self, Failure> {
        Ok(match mode {
            0 => Self::Bicubic(BicubicResizePlan::try_new(
                source, output, anchor, policy, budget,
            )?),
            2 => Self::Lanczos(LanczosResizePlan::try_new2(
                source, output, anchor, policy, budget,
            )?),
            3 => Self::Lanczos(LanczosResizePlan::try_new3(
                source, output, anchor, policy, budget,
            )?),
            _ => unreachable!(),
        })
    }
    fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Bicubic(p) => p.capacity_bytes(),
            Self::Lanczos(p) => p.capacity_bytes(),
        }
    }
    fn scratch_elements(&self) -> usize {
        match self {
            Self::Bicubic(p) => p.scratch_elements(),
            Self::Lanczos(p) => p.scratch_elements(),
        }
        .unwrap()
    }
    fn execute(
        &self,
        source: ImageView<Rgba8>,
        output: ImageViewMut<Rgba8>,
        scratch: &mut [f64],
    ) -> Result<(), Failure> {
        match self {
            Self::Bicubic(p) => {
                resize_bicubic_rgba8_with_plan_and_scratch_into(source, output, p, scratch)
            }
            Self::Lanczos(p) => {
                resize_lanczos_rgba8_with_plan_and_scratch_into(source, output, p, scratch)
            }
        }
    }
}

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}
fn source_bytes(d: ImageDimensions) -> Vec<u8> {
    (0..d.storage_len::<Rgba8>().unwrap())
        .map(|i| ((i * 71 + i / 19 * 113) % 256) as u8)
        .collect()
}

#[test]
fn fallible_paths_match_landed_outputs_without_execution_allocations() {
    let anchors = [
        ResizeAnchor::TopLeft,
        ResizeAnchor::Top,
        ResizeAnchor::TopRight,
        ResizeAnchor::Left,
        ResizeAnchor::Center,
        ResizeAnchor::Right,
        ResizeAnchor::BottomLeft,
        ResizeAnchor::Bottom,
        ResizeAnchor::BottomRight,
    ];
    for (sw, sh, ow, oh) in [
        (7, 5, 11, 9),
        (11, 9, 3, 2),
        (5, 7, 5, 7),
        (11, 9, 11, 3),
        (11, 9, 3, 9),
        (1, 1, 3, 5),
        (101, 100, 31, 47),
    ] {
        let source_dimensions = dimensions(sw, sh);
        let output_dimensions = dimensions(ow, oh);
        let bytes = source_bytes(source_dimensions);
        let source = ImageView::packed(&bytes, source_dimensions).unwrap();
        for mode in [0, 2, 3] {
            for anchor in anchors {
                for policy in [SupportPolicy::Fixed, SupportPolicy::ScaleAware] {
                    let required =
                        Plan::required(mode, source_dimensions, output_dimensions, policy).unwrap();
                    let mut budget = CapacityBudget::new(required);
                    let plan = Plan::new(
                        mode,
                        source_dimensions,
                        output_dimensions,
                        anchor,
                        policy,
                        &mut budget,
                    )
                    .unwrap();
                    assert_eq!(budget.used(), plan.capacity_bytes());
                    let scratch_len = plan.scratch_elements();
                    let expected_scratch = if sw != ow
                        && sh != oh
                        && sw > ow
                        && sw * sh >= 10_000
                        && policy == SupportPolicy::ScaleAware
                    {
                        sh as usize * ow as usize * 4
                    } else {
                        0
                    };
                    assert_eq!(scratch_len, expected_scratch);
                    let mut scratch = budget.vector::<f64>(scratch_len).unwrap();
                    scratch.resize(scratch_len, f64::NAN);
                    assert_eq!(
                        budget.used(),
                        plan.capacity_bytes() + scratch.capacity() as u64 * 8
                    );
                    assert!(budget.used() <= required);
                    let mut expected = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
                    let mut actual = expected.clone();
                    let expected_view =
                        ImageViewMut::packed(&mut expected, output_dimensions).unwrap();
                    match mode {
                        0 => resize_bicubic_rgba8_into(source, expected_view, anchor, policy),
                        2 => resize_lanczos2_rgba8_into(source, expected_view, anchor, policy),
                        3 => resize_lanczos3_rgba8_into(source, expected_view, anchor, policy),
                        _ => unreachable!(),
                    }
                    let actual_view = ImageViewMut::packed(&mut actual, output_dimensions).unwrap();
                    let before = ALLOCATIONS.with(Cell::get);
                    let result = plan.execute(source, actual_view, &mut scratch);
                    let after = ALLOCATIONS.with(Cell::get);
                    result.unwrap();
                    assert_eq!(before, after, "execution allocated");
                    assert!(scratch.iter().all(|value| value.is_finite()));
                    assert_eq!(actual, expected, "mode {mode}, {source_dimensions:?} -> {output_dimensions:?}, {anchor:?}, {policy:?}");
                }
            }
        }
    }
}

#[test]
fn budget_preflight_rejects_before_allocating_and_identity_needs_no_plan_heap() {
    for mode in [0, 2, 3] {
        let source = dimensions(101, 100);
        let output = dimensions(31, 47);
        let policy = SupportPolicy::ScaleAware;
        let required = Plan::required(mode, source, output, policy).unwrap();
        let mut budget = CapacityBudget::new(required - 1);
        let before = ALLOCATIONS.with(Cell::get);
        let result = Plan::new(
            mode,
            source,
            output,
            ResizeAnchor::Center,
            policy,
            &mut budget,
        );
        let after = ALLOCATIONS.with(Cell::get);
        assert_eq!(result.err().unwrap().code, ErrorCode::MemoryLimit);
        assert_eq!(budget.used(), 0);
        assert_eq!(before, after);
        let mut zero = CapacityBudget::new(0);
        let identity = Plan::new(
            mode,
            source,
            source,
            ResizeAnchor::Center,
            policy,
            &mut zero,
        )
        .unwrap();
        assert_eq!(Plan::required(mode, source, source, policy).unwrap(), 0);
        assert_eq!(identity.capacity_bytes(), 0);
        assert_eq!(identity.scratch_elements(), 0);
    }
    // This must fail during arithmetic, without trying to visit billions of output coordinates.
    let enormous = dimensions(u32::MAX, u32::MAX);
    let output = dimensions(u32::MAX - 1, u32::MAX - 1);
    assert_eq!(
        LanczosResizePlan::required_bytes(
            enormous,
            output,
            NonZeroU32::new(u32::MAX).unwrap(),
            SupportPolicy::Fixed
        )
        .unwrap_err()
        .code,
        ErrorCode::MemoryLimit
    );
}

#[test]
fn every_plan_and_scratch_reservation_failure_is_returned_and_retry_works() {
    let source = dimensions(101, 100);
    let output = dimensions(3, 2);
    for mode in [0, 2, 3] {
        let required = Plan::required(mode, source, output, SupportPolicy::ScaleAware).unwrap();
        // Two outer vectors, one allocation per output axis coordinate, then full-call scratch.
        for allocation in 0..(2 + output.width() + output.height() + 1) as usize {
            let mut budget = CapacityBudget::new(required);
            FAIL_AFTER.with(|remaining| remaining.set(Some(allocation)));
            let result = Plan::new(
                mode,
                source,
                output,
                ResizeAnchor::Center,
                SupportPolicy::ScaleAware,
                &mut budget,
            )
            .and_then(|plan| {
                budget
                    .vector::<f64>(plan.scratch_elements())
                    .map(|scratch| (plan, scratch))
            });
            FAIL_AFTER.with(|remaining| remaining.set(None));
            assert_eq!(
                result.err().unwrap().code,
                ErrorCode::WasmMemoryUnavailable,
                "allocation {allocation}"
            );
            let mut retry_budget = CapacityBudget::new(required);
            let retry = Plan::new(
                mode,
                source,
                output,
                ResizeAnchor::Center,
                SupportPolicy::ScaleAware,
                &mut retry_budget,
            )
            .unwrap();
            retry_budget
                .vector::<f64>(retry.scratch_elements())
                .unwrap();
        }
    }
}

#[test]
fn short_scratch_and_wrong_shape_leave_output_untouched() {
    let source_dimensions = dimensions(101, 100);
    let output_dimensions = dimensions(31, 47);
    let bytes = source_bytes(source_dimensions);
    let source = ImageView::packed(&bytes, source_dimensions).unwrap();
    for mode in [0, 2, 3] {
        let mut budget = CapacityBudget::new(u64::MAX);
        let plan = Plan::new(
            mode,
            source_dimensions,
            output_dimensions,
            ResizeAnchor::Center,
            SupportPolicy::ScaleAware,
            &mut budget,
        )
        .unwrap();
        let mut scratch = vec![0.; plan.scratch_elements() - 1];
        let mut output = vec![173; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let result = plan.execute(
            source,
            ImageViewMut::packed(&mut output, output_dimensions).unwrap(),
            &mut scratch,
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::MemoryLimit);
        assert!(output.iter().all(|&byte| byte == 173));
        let wrong = dimensions(47, 31);
        let result = plan.execute(
            source,
            ImageViewMut::packed(&mut output, wrong).unwrap(),
            &mut scratch,
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::InvalidImage);
        assert!(output.iter().all(|&byte| byte == 173));
    }
}

#[test]
fn identity_plans_support_existing_row_calls_and_reject_extra_backing_storage() {
    use ditherette_wasm::{
        image::RowStride,
        prod::resize::scalar::{
            bicubic::resize_bicubic_rgba8_rows_with_plan_into,
            lanczos::resize_lanczos_rgba8_rows_with_plan_into,
        },
    };
    let dimensions = dimensions(5, 7);
    let bytes = source_bytes(dimensions);
    let source = ImageView::packed(&bytes, dimensions).unwrap();
    for mode in [0, 2, 3] {
        let mut budget = CapacityBudget::new(0);
        let plan = Plan::new(
            mode,
            dimensions,
            dimensions,
            ResizeAnchor::Center,
            SupportPolicy::Fixed,
            &mut budget,
        )
        .unwrap();
        let mut band = [0; 40];
        let view = ImageViewMut::packed(&mut band, ImageDimensions::new(5, 2).unwrap()).unwrap();
        match &plan {
            Plan::Bicubic(p) => resize_bicubic_rgba8_rows_with_plan_into(source, view, p, 3),
            Plan::Lanczos(p) => resize_lanczos_rgba8_rows_with_plan_into(source, view, p, 3),
        }
        assert_eq!(&band, &bytes[60..100]);
        let mut extra = bytes.clone();
        extra.push(0);
        let stride = RowStride::packed::<Rgba8>(dimensions).unwrap();
        let extra_source = ImageView::new(&extra, dimensions, stride).unwrap();
        let mut output = vec![173; bytes.len()];
        let result = plan.execute(
            extra_source,
            ImageViewMut::packed(&mut output, dimensions).unwrap(),
            &mut [],
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::InvalidImage);
        assert!(output.iter().all(|&byte| byte == 173));
        output.push(173);
        let result = plan.execute(
            source,
            ImageViewMut::new(&mut output, dimensions, stride).unwrap(),
            &mut [],
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::InvalidImage);
        assert!(output.iter().all(|&byte| byte == 173));
    }
}
