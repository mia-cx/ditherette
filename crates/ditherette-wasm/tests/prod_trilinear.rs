use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Oklab32, Rgba8, RowStride},
    prod::resize::scalar::{
        bilinear::alignment::ResizeAnchor as ProdAnchor,
        trilinear::{resize_trilinear_into, PreparedTrilinear},
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecAnchor,
        scalar::trilinear::resize_trilinear_into as oracle,
    },
};

const ANCHORS: [(ProdAnchor, SpecAnchor); 9] = [
    (ProdAnchor::TopLeft, SpecAnchor::TopLeft),
    (ProdAnchor::Top, SpecAnchor::Top),
    (ProdAnchor::TopRight, SpecAnchor::TopRight),
    (ProdAnchor::Left, SpecAnchor::Left),
    (ProdAnchor::Center, SpecAnchor::Center),
    (ProdAnchor::Right, SpecAnchor::Right),
    (ProdAnchor::BottomLeft, SpecAnchor::BottomLeft),
    (ProdAnchor::Bottom, SpecAnchor::Bottom),
    (ProdAnchor::BottomRight, SpecAnchor::BottomRight),
];

#[test]
fn exact_rgba_mips_lod_anchors_and_strided_rows() {
    for (sw, sh, ow, oh) in [
        (1, 1, 7, 9),
        (7, 9, 7, 9),
        (7, 9, 3, 4),
        (7, 9, 2, 3),
        (8, 8, 2, 2),
        (17, 13, 1, 1),
        (31, 1, 3, 1),
        (1, 31, 1, 3),
        (31, 3, 2, 17),
        (2, 17, 31, 3),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let source_stride = sw as usize * 4 + 3;
        let output_stride = ow as usize * 4 + 5;
        let mut bytes = vec![201; source_stride * sh as usize];
        for y in 0..sh as usize {
            for x in 0..sw as usize * 4 {
                bytes[y * source_stride + x] = if x % 4 == 3 {
                    [0, 1, 127, 254, 255][(x / 4 + y) % 5]
                } else {
                    ((x * 73 + y * 19) % 256) as u8
                };
            }
        }
        for (anchor, spec_anchor) in ANCHORS {
            let mut expected = vec![211; output_stride * oh as usize];
            let mut actual = expected.clone();
            oracle(
                ImageView::<Rgba8>::new(&bytes, source, RowStride::new(source_stride).unwrap())
                    .unwrap(),
                ImageViewMut::new(
                    &mut expected,
                    output,
                    RowStride::new(output_stride).unwrap(),
                )
                .unwrap(),
                spec_anchor,
            );
            resize_trilinear_into(
                ImageView::<Rgba8>::new(&bytes, source, RowStride::new(source_stride).unwrap())
                    .unwrap(),
                ImageViewMut::new(&mut actual, output, RowStride::new(output_stride).unwrap())
                    .unwrap(),
                anchor,
            );
            assert_eq!(actual, expected, "{sw}x{sh}->{ow}x{oh}, {anchor:?}");
            let budget = PreparedTrilinear::<Rgba8>::required_bytes(source, output).unwrap();
            let mut prepared =
                PreparedTrilinear::<Rgba8>::try_new(source, output, anchor, budget).unwrap();
            assert_eq!(prepared.capacity_bytes(), budget);
            actual.fill(211);
            let allocations = ALLOCATIONS.with(std::cell::Cell::get);
            prepared
                .execute(
                    ImageView::<Rgba8>::new(&bytes, source, RowStride::new(source_stride).unwrap())
                        .unwrap(),
                    ImageViewMut::new(&mut actual, output, RowStride::new(output_stride).unwrap())
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(ALLOCATIONS.with(std::cell::Cell::get), allocations);
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn intermediate_float_storage_rounds_identically() {
    let source = ImageDimensions::new(7, 9).unwrap();
    let output = ImageDimensions::new(2, 3).unwrap();
    let bytes: Vec<f32> = (0..7 * 9 * 3).map(|x| x as f32 / 73.0 - 0.7).collect();
    for (anchor, spec_anchor) in ANCHORS {
        let mut expected = vec![0.0f32; 2 * 3 * 3];
        let mut actual = expected.clone();
        oracle(
            ImageView::<Oklab32>::packed(&bytes, source).unwrap(),
            ImageViewMut::packed(&mut expected, output).unwrap(),
            spec_anchor,
        );
        resize_trilinear_into(
            ImageView::<Oklab32>::packed(&bytes, source).unwrap(),
            ImageViewMut::packed(&mut actual, output).unwrap(),
            anchor,
        );
        assert_eq!(
            actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
        let mut prepared =
            PreparedTrilinear::<Oklab32>::try_new(source, output, anchor, 1_000_000).unwrap();
        prepared
            .execute(
                ImageView::packed(&bytes, source).unwrap(),
                ImageViewMut::packed(&mut actual, output).unwrap(),
            )
            .unwrap();
        assert_eq!(
            actual.iter().map(|x| x.to_bits()).collect::<Vec<_>>(),
            expected.iter().map(|x| x.to_bits()).collect::<Vec<_>>()
        );
    }
}

#[test]
fn byte_rounding_happens_at_each_mip_not_only_at_final_output() {
    let source = ImageDimensions::new(4, 1).unwrap();
    let output = ImageDimensions::new(1, 1).unwrap();
    let pixels = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1];
    let mut actual = [0; 4];
    let mut prepared =
        PreparedTrilinear::<Rgba8>::try_new(source, output, ProdAnchor::Center, 10_000).unwrap();
    prepared
        .execute(
            ImageView::packed(&pixels, source).unwrap(),
            ImageViewMut::packed(&mut actual, output).unwrap(),
        )
        .unwrap();
    // First mip rounds [0, 0.5] to [0, 1]. The second rounds 0.5 to 1.
    // A single unrounded 4-pixel average would incorrectly produce zero.
    assert_eq!(actual, [1; 4]);
}

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[test]
fn execution_validates_before_writes_and_refills_every_chain() {
    use ditherette_wasm::prod::contract::error::ErrorCode;
    let source = ImageDimensions::new(7, 9).unwrap();
    let output = ImageDimensions::new(2, 3).unwrap();
    let mut prepared =
        PreparedTrilinear::<Rgba8>::try_new(source, output, ProdAnchor::Center, 1_000_000).unwrap();
    let mut target = vec![211; 2 * 3 * 4];
    let wrong = ImageDimensions::new(1, 1).unwrap();
    assert_eq!(
        prepared
            .execute(
                ImageView::packed(&[0; 4], wrong).unwrap(),
                ImageViewMut::packed(&mut target, output).unwrap()
            )
            .unwrap_err()
            .code,
        ErrorCode::InvalidImage
    );
    assert_eq!(target, vec![211; 2 * 3 * 4]);
    for value in [73, 191] {
        let pixels = vec![value; 7 * 9 * 4];
        prepared
            .execute(
                ImageView::packed(&pixels, source).unwrap(),
                ImageViewMut::packed(&mut target, output).unwrap(),
            )
            .unwrap();
        assert!(target.iter().all(|byte| *byte == value));
    }
}
thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
    static FAIL_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
}
struct TestAllocator;
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator;
unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|value| value.set(value.get() + 1));
        let fail = FAIL_AFTER.with(|value| match value.get() {
            Some(0) => {
                value.set(None);
                true
            }
            Some(n) => {
                value.set(Some(n - 1));
                false
            }
            None => false,
        });
        if fail {
            return std::ptr::null_mut();
        }
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            LIVE_BYTES.with(|value| value.set(value.get().wrapping_add(layout.size() as isize)));
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.with(|value| value.set(value.get().wrapping_sub(layout.size() as isize)));
        System.dealloc(pointer, layout);
    }
}

#[test]
fn every_preparation_failure_releases_chains_and_exact_budget_rejects_before_allocation() {
    use ditherette_wasm::prod::contract::error::ErrorCode;
    for (sw, sh, ow, oh) in [(1, 1, 7, 9), (8, 8, 2, 2), (7, 9, 2, 3), (31, 1, 3, 1)] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let required = PreparedTrilinear::<Rgba8>::required_bytes(source, output).unwrap();
        let allocations = ALLOCATIONS.with(Cell::get);
        let mut prepared =
            PreparedTrilinear::<Rgba8>::try_new(source, output, ProdAnchor::Center, required)
                .unwrap();
        let count = ALLOCATIONS.with(Cell::get) - allocations;
        let input = vec![73; (sw * sh * 4) as usize];
        let mut actual = vec![0; (ow * oh * 4) as usize];
        let before = ALLOCATIONS.with(Cell::get);
        prepared
            .execute(
                ImageView::packed(&input, source).unwrap(),
                ImageViewMut::packed(&mut actual, output).unwrap(),
            )
            .unwrap();
        assert_eq!(ALLOCATIONS.with(Cell::get), before);
        assert!(actual.iter().all(|x| *x == 73));
        drop(prepared);
        let before = ALLOCATIONS.with(Cell::get);
        assert!(
            matches!(PreparedTrilinear::<Rgba8>::try_new(source,output,ProdAnchor::Center,required-1),Err(error) if error.code==ErrorCode::MemoryLimit)
        );
        assert_eq!(ALLOCATIONS.with(Cell::get), before);
        for after in 0..count {
            let live = LIVE_BYTES.with(Cell::get);
            FAIL_AFTER.with(|value| value.set(Some(after)));
            let result =
                PreparedTrilinear::<Rgba8>::try_new(source, output, ProdAnchor::Center, required);
            FAIL_AFTER.with(|value| value.set(None));
            assert!(matches!(result,Err(error) if error.code==ErrorCode::WasmMemoryUnavailable));
            assert_eq!(
                LIVE_BYTES.with(Cell::get),
                live,
                "leak at reservation {after}"
            );
        }
    }
}
