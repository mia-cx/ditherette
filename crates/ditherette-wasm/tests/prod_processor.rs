use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{Anchor, Output, ResizePolicy, Support},
        },
        pipeline::processor::{Boundary, Processor, ResizeRequest},
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
    static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
    static WATCH_ALLOCATION: Cell<usize> = const { Cell::new(0) };
    static WATCHED_LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
}
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator;

unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        WATCH_ALLOCATION.with(|size| {
            if size.get() == layout.size() {
                WATCHED_LIVE_BYTES.with(|observed| observed.set(LIVE_BYTES.with(Cell::get)));
            }
        });
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
            let pointer = System.alloc(layout);
            if !pointer.is_null() {
                LIVE_BYTES
                    .with(|bytes| bytes.set(bytes.get().wrapping_add(layout.size() as isize)));
            }
            pointer
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.with(|bytes| bytes.set(bytes.get().wrapping_sub(layout.size() as isize)));
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

fn request() -> ResizeRequest {
    ResizeRequest {
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
    let mut probe = Processor::new(1_000_000, 512).unwrap();
    probe.resize(request(), &mut io()).unwrap();
    probe.peak_capacity_bytes()
}

#[test]
fn wider_diffusion_drops_idle_rows_before_reserving_their_replacement() {
    use ditherette_wasm::{
        image::contracts::PaletteEntry,
        prod::{
            contract::request::{
                AlphaPolicy, Diffusion, DiffusionFeedback, DitherPolicy, MatchPolicy, Placement,
            },
            palette::PreparedPalette,
            pipeline::quantize::{QuantizeBoundary, QuantizeRequest},
        },
    };
    struct Input(usize);
    impl QuantizeBoundary for Input {
        type Output = ();
        fn input_len(&mut self) -> Result<usize, Failure> {
            Ok(self.0 * 4)
        }
        fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
            destination.fill(255);
            Ok(())
        }
        fn complete(
            &mut self,
            _: &[u8],
            _: ImageDimensions,
            _: &PreparedPalette,
        ) -> Result<(), Failure> {
            Ok(())
        }
    }
    let palette = [PaletteEntry::Color { rgb: [255; 3] }];
    let request = |width| QuantizeRequest {
        source_width: width,
        source_height: 1,
        palette: &palette,
        alpha: AlphaPolicy::Premultiplied {},
        matching: MatchPolicy::SrgbEuclidean,
    };
    let dither = DitherPolicy::Diffusion {
        kernel: Diffusion::Sierra,
        feedback: DiffusionFeedback::Matching,
        strength: 1.0,
        serpentine: true,
        placement: Placement::Everywhere {},
    };
    let mut probe = Processor::new(1 << 20, 0).unwrap();
    probe
        .dither_and_quantize(request(20), dither, &mut Input(20))
        .unwrap();
    let limit = probe.peak_capacity_bytes();
    probe.dispose().unwrap();
    let mut processor = Processor::new(limit, 0).unwrap();
    processor
        .dither_and_quantize(request(10), dither, &mut Input(10))
        .unwrap();
    let before = LIVE_BYTES.with(Cell::get);
    WATCH_ALLOCATION.with(|size| size.set(20 * 3 * 12));
    processor
        .dither_and_quantize(request(20), dither, &mut Input(20))
        .unwrap();
    WATCH_ALLOCATION.with(|size| size.set(0));
    // Source and indices grow by 50 bytes. The old three rows release 360 bytes.
    assert_eq!(WATCHED_LIVE_BYTES.with(Cell::get), before + 50 - 360);
    assert_eq!(processor.peak_capacity_bytes(), limit);
}

#[test]
fn public_area_and_bilinear_dispatch_preserves_landed_output() {
    use ditherette_wasm::prod::resize::scalar::{area, bilinear};
    for policy in [
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
    ] {
        let mut request = request();
        request.output.resize = policy;
        let mut input = io();
        let mut expected = vec![0; 24];
        let source = ImageView::packed(&input.input, ImageDimensions::new(2, 1).unwrap()).unwrap();
        let output =
            ImageViewMut::packed(&mut expected, ImageDimensions::new(3, 2).unwrap()).unwrap();
        match policy {
            ResizePolicy::Area {} => area::resize_area_rgba8_into(source, output),
            _ => bilinear::resize_bilinear_rgba8_into(
                source,
                output,
                bilinear::alignment::ResizeAnchor::Center,
            ),
        }
        let mut processor = Processor::new(1_000_000, 512).unwrap();
        assert_eq!(processor.resize(request, &mut input).unwrap(), expected);
    }
}

#[test]
fn public_trilinear_matches_frozen_bytes_and_counts_its_record_once() {
    use ditherette_wasm::prod::resize::scalar::trilinear::PreparedTrilinear;
    use ditherette_wasm::spec::resize::scalar::trilinear::resize_trilinear_into;
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
    // Independent odd-mip witness: area gives [0,170], bilinear [43,85,128],
    // then blending against 85 at log2(3)-1 rounds to [68,85,103].
    for (width, height) in [(3, 1), (1, 3)] {
        for (index, (_, anchor)) in anchors.iter().enumerate() {
            let mut output = [0; 4];
            resize_trilinear_into(
                ImageView::<Rgba8>::packed(
                    &[0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255],
                    ImageDimensions::new(width, height).unwrap(),
                )
                .unwrap(),
                ImageViewMut::packed(&mut output, ImageDimensions::new(1, 1).unwrap()).unwrap(),
                *anchor,
            );
            assert_eq!(
                output,
                [[68; 4], [85; 4], [103; 4]][if width == 1 { index / 3 } else { index % 3 }]
            );
        }
    }
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
        let pixels: Vec<u8> = (0..sw * sh * 4)
            .map(|i| ((i * 73 + i / 7) % 256) as u8)
            .collect();
        for (anchor, reference_anchor) in anchors {
            let mut expected = vec![0; (ow * oh * 4) as usize];
            resize_trilinear_into(
                ImageView::<Rgba8>::packed(&pixels, source).unwrap(),
                ImageViewMut::packed(&mut expected, output).unwrap(),
                reference_anchor,
            );
            let heap = if source == output {
                0
            } else {
                PreparedTrilinear::<Rgba8>::required_bytes(source, output).unwrap()
                    - std::mem::size_of::<PreparedTrilinear<Rgba8>>() as u64
            };
            let mut processor = Processor::new(1_000_000, 512).unwrap();
            let request = ResizeRequest {
                source_width: sw,
                source_height: sh,
                output: Output {
                    width: ow,
                    height: oh,
                    resize: ResizePolicy::Trilinear { anchor },
                },
            };
            let mut input = Io {
                input: pixels.clone(),
                ..Io::default()
            };
            let result = processor.resize(request, &mut input).unwrap();
            let required = processor.peak_capacity_bytes();
            let heap_without_record = Processor::bookkeeping_bytes(512)
                + pixels.len() as u64
                + expected.len() as u64
                + heap;
            assert!(required > heap_without_record);
            let mut exact = Processor::new(required, 512).unwrap();
            assert_eq!(exact.resize(request, &mut input).unwrap(), expected);
            assert_eq!(result, expected, "{sw}x{sh}->{ow}x{oh} {anchor:?}");
            assert_eq!(processor.peak_capacity_bytes(), required);
            assert_eq!(input.input, pixels);
            processor.dispose().unwrap();
            assert_eq!(result, expected);
            let mut short = Processor::new(required - 1, 512).unwrap();
            input.copy_calls = 0;
            let allocations = ALLOCATIONS.with(Cell::get);
            assert_eq!(
                short.resize(request, &mut input).unwrap_err().code,
                ErrorCode::MemoryLimit
            );
            assert_eq!(ALLOCATIONS.with(Cell::get), allocations);
            assert_eq!(input.copy_calls, 0);
        }
    }
}

#[test]
fn resize_reservation_failures_release_every_owned_byte_and_recover() {
    for policy in [
        ResizePolicy::Trilinear {
            anchor: Anchor::Center,
        },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
    ] {
        for (sw, sh, ow, oh) in [
            (3, 2, 5, 4),
            (3, 5, 3, 2),
            (5, 3, 2, 3),
            (2, 2, 4, 4),
            (101, 100, 3, 2),
        ] {
            let request = ResizeRequest {
                source_width: sw,
                source_height: sh,
                output: Output {
                    width: ow,
                    height: oh,
                    resize: policy,
                },
            };
            let mut input = Io {
                input: vec![73; (sw * sh * 4) as usize],
                ..Io::default()
            };
            let mut processor = Processor::new(1_000_000, 512).unwrap();
            let before = ALLOCATIONS.with(Cell::get);
            let expected = processor.resize(request, &mut input).unwrap();
            let reservations = ALLOCATIONS.with(Cell::get) - before - 1; // Durable fixture output is caller-owned.
            let peak = processor.peak_capacity_bytes();
            let mut exact = Processor::new(peak, 512).unwrap();
            assert_eq!(exact.resize(request, &mut input).unwrap(), expected);
            let mut short = Processor::new(peak - 1, 512).unwrap();
            let before = ALLOCATIONS.with(Cell::get);
            assert_eq!(
                short.resize(request, &mut input).unwrap_err().code,
                ErrorCode::MemoryLimit
            );
            assert_eq!(ALLOCATIONS.with(Cell::get), before);
            for fail_after in 0..reservations {
                processor = Processor::new(peak, 512).unwrap();
                input.copy_calls = 0;
                input.complete_calls = 0;
                let live = LIVE_BYTES.with(Cell::get);
                ALLOCATION_FAILURE.with(|remaining| remaining.set(Some(fail_after)));
                let failure = processor.resize(request, &mut input).unwrap_err();
                ALLOCATION_FAILURE.with(|remaining| remaining.set(None));
                assert_eq!(failure.code, ErrorCode::WasmMemoryUnavailable);
                assert_eq!(
                    LIVE_BYTES.with(Cell::get),
                    live,
                    "partial preparation leaked at allocation {fail_after}"
                );
                assert_eq!((input.copy_calls, input.complete_calls), (0, 0));
                assert_eq!(processor.resize(request, &mut input).unwrap(), expected);
            }
            for complete in [false, true] {
                processor = Processor::new(peak, 512).unwrap();
                input.fail_copy = !complete;
                input.fail_complete = complete;
                let live = LIVE_BYTES.with(Cell::get);
                assert!(processor.resize(request, &mut input).is_err());
                assert_eq!(LIVE_BYTES.with(Cell::get), live);
            }
        }
    }
}

#[test]
fn public_convolution_preserves_landed_bytes_and_reports_frozen_differences() {
    use ditherette_wasm::{
        prod::resize::scalar::{bicubic, convolution, lanczos},
        spec::resize::scalar::{
            bicubic as oracle_bicubic, convolution as oracle_convolution, lanczos as oracle_lanczos,
        },
    };
    let anchors = [
        (
            Anchor::TopLeft,
            ResizeAnchor::TopLeft,
            convolution::ResizeAnchor::TopLeft,
        ),
        (
            Anchor::Top,
            ResizeAnchor::Top,
            convolution::ResizeAnchor::Top,
        ),
        (
            Anchor::TopRight,
            ResizeAnchor::TopRight,
            convolution::ResizeAnchor::TopRight,
        ),
        (
            Anchor::Left,
            ResizeAnchor::Left,
            convolution::ResizeAnchor::Left,
        ),
        (
            Anchor::Center,
            ResizeAnchor::Center,
            convolution::ResizeAnchor::Center,
        ),
        (
            Anchor::Right,
            ResizeAnchor::Right,
            convolution::ResizeAnchor::Right,
        ),
        (
            Anchor::BottomLeft,
            ResizeAnchor::BottomLeft,
            convolution::ResizeAnchor::BottomLeft,
        ),
        (
            Anchor::Bottom,
            ResizeAnchor::Bottom,
            convolution::ResizeAnchor::Bottom,
        ),
        (
            Anchor::BottomRight,
            ResizeAnchor::BottomRight,
            convolution::ResizeAnchor::BottomRight,
        ),
    ];
    let mut maxima = [0_u32; 3];
    let mut differing = [0_usize; 3];
    for (sw, sh, ow, oh) in [
        (1, 1, 9, 7),
        (7, 5, 7, 5),
        (7, 5, 3, 2),
        (3, 2, 7, 5),
        (9, 1, 1, 9),
        (1, 9, 9, 1),
        (101, 100, 31, 47),
        (32768, 1, 1, 1),
        (1, 1, 16384, 1),
    ] {
        let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
        let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
        let bytes: Vec<u8> = (0..sw * sh * 4)
            .map(|x| {
                if x % 4 == 3 {
                    [0, 1, 127, 254, 255][(x / 4) as usize % 5]
                } else {
                    (x * 73) as u8
                }
            })
            .collect();
        for (anchor, oracle_anchor, landed_anchor) in anchors {
            for (support, oracle_support, landed_support) in [
                (
                    Support::Fixed,
                    oracle_convolution::SupportPolicy::Fixed,
                    convolution::SupportPolicy::Fixed,
                ),
                (
                    Support::ScaleAware,
                    oracle_convolution::SupportPolicy::ScaleAware,
                    convolution::SupportPolicy::ScaleAware,
                ),
            ] {
                for (mode, policy) in [
                    (0, ResizePolicy::Bicubic { anchor, support }),
                    (1, ResizePolicy::Lanczos2 { anchor, support }),
                    (2, ResizePolicy::Lanczos3 { anchor, support }),
                ] {
                    let mut landed = vec![0; (ow * oh * 4) as usize];
                    let mut oracle = landed.clone();
                    let source = ImageView::<Rgba8>::packed(&bytes, source_dimensions).unwrap();
                    let target = ImageViewMut::packed(&mut landed, output_dimensions).unwrap();
                    match mode {
                        0 => bicubic::resize_bicubic_rgba8_into(
                            source,
                            target,
                            landed_anchor,
                            landed_support,
                        ),
                        1 => lanczos::resize_lanczos2_rgba8_into(
                            source,
                            target,
                            landed_anchor,
                            landed_support,
                        ),
                        _ => lanczos::resize_lanczos3_rgba8_into(
                            source,
                            target,
                            landed_anchor,
                            landed_support,
                        ),
                    }
                    let target = ImageViewMut::packed(&mut oracle, output_dimensions).unwrap();
                    match mode {
                        0 => oracle_bicubic::resize_bicubic_into(
                            source,
                            target,
                            oracle_anchor,
                            oracle_support,
                        ),
                        1 => oracle_lanczos::resize_lanczos2_into(
                            source,
                            target,
                            oracle_anchor,
                            oracle_support,
                        ),
                        _ => oracle_lanczos::resize_lanczos3_into(
                            source,
                            target,
                            oracle_anchor,
                            oracle_support,
                        ),
                    }
                    let mut io = Io {
                        input: bytes.clone(),
                        ..Io::default()
                    };
                    let mut processor = Processor::new(20_000_000, 512).unwrap();
                    let actual = processor
                        .resize(
                            ResizeRequest {
                                source_width: sw,
                                source_height: sh,
                                output: Output {
                                    width: ow,
                                    height: oh,
                                    resize: policy,
                                },
                            },
                            &mut io,
                        )
                        .unwrap();
                    assert_eq!(
                        actual, landed,
                        "landed output changed for {policy:?} {sw}x{sh}->{ow}x{oh}"
                    );
                    assert_eq!(io.input, bytes);
                    for (actual, oracle) in actual.chunks_exact(4).zip(oracle.chunks_exact(4)) {
                        let squared = actual
                            .iter()
                            .zip(oracle)
                            .map(|(a, b)| u32::from(a.abs_diff(*b)).pow(2))
                            .sum();
                        maxima[mode] = maxima[mode].max(squared);
                        differing[mode] += usize::from(squared != 0);
                    }
                }
            }
        }
    }
    // Diagnostic only. This test gates exact preservation of landed bytes, not approval of new approximation.
    println!("bicubic/lanczos2/lanczos3 frozen max squared RGBA distances {maxima:?}; differing pixels {differing:?}");
}

#[test]
fn public_filters_keep_frozen_reference_bounds_across_alpha_anchors_and_extremes() {
    use ditherette_wasm::spec::resize::scalar::{area, bilinear};
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
    let mut maxima = [0u32; 2];
    for (sw, sh, ow, oh) in [
        (1, 1, 9, 7),
        (7, 5, 7, 5),
        (7, 5, 3, 2),
        (3, 2, 7, 5),
        (9, 1, 1, 9),
        (1, 9, 9, 1),
        (31, 3, 2, 17),
        (2, 17, 31, 3),
        (32768, 1, 1, 1),
        (1, 1, 16384, 1),
    ] {
        let source = ImageDimensions::new(sw, sh).unwrap();
        let output = ImageDimensions::new(ow, oh).unwrap();
        let bytes: Vec<u8> = (0..sw * sh * 4)
            .map(|x| {
                if x % 4 == 3 {
                    [0, 1, 127, 254, 255][(x / 4) as usize % 5]
                } else {
                    (x * 73) as u8
                }
            })
            .collect();
        for (anchor, oracle_anchor) in anchors {
            for (mode, policy) in [
                (0, ResizePolicy::Area {}),
                (1, ResizePolicy::Bilinear { anchor }),
            ] {
                let mut expected = vec![0; (ow * oh * 4) as usize];
                let input = ImageView::<Rgba8>::packed(&bytes, source).unwrap();
                let target = ImageViewMut::packed(&mut expected, output).unwrap();
                if mode == 0 {
                    area::resize_area_into(input, target);
                } else {
                    bilinear::resize_bilinear_into(input, target, oracle_anchor);
                }
                let mut io = Io {
                    input: bytes.clone(),
                    ..Io::default()
                };
                let mut processor = Processor::new(10_000_000, 512).unwrap();
                let actual = processor
                    .resize(
                        ResizeRequest {
                            source_width: sw,
                            source_height: sh,
                            output: Output {
                                width: ow,
                                height: oh,
                                resize: policy,
                            },
                        },
                        &mut io,
                    )
                    .unwrap();
                assert_eq!(io.input, bytes);
                for (actual, expected) in actual.chunks_exact(4).zip(expected.chunks_exact(4)) {
                    let distance_squared = actual
                        .iter()
                        .zip(expected)
                        .map(|(a, b)| u32::from(a.abs_diff(*b)).pow(2))
                        .sum::<u32>();
                    maxima[mode] = maxima[mode].max(distance_squared);
                    assert!(
                        distance_squared <= 4,
                        "{policy:?} {sw}x{sh}->{ow}x{oh}: {actual:?} vs {expected:?}"
                    );
                }
            }
        }
    }
    println!(
        "frozen-reference maximum squared RGBA distances: area={}, bilinear={}",
        maxima[0], maxima[1]
    );
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
    // One preparation record, two coordinate maps, then source and output buffers.
    for successful_allocations in 0..5 {
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
    let request = ResizeRequest {
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
    for successful_allocations in 0..6 {
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
            ResizeRequest {
                source_width: 0,
                ..request()
            },
            ErrorPath::SourceWidth,
        ),
        (
            ResizeRequest {
                source_height: 32769,
                ..request()
            },
            ErrorPath::SourceHeight,
        ),
        (
            ResizeRequest {
                output: Output {
                    width: 0,
                    ..request().output
                },
                ..request()
            },
            ErrorPath::OutputWidth,
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
            let request = ResizeRequest {
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
