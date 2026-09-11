use ditherette_wasm::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageDimensions, ImageView, ImageViewMut, PaletteIndex8, Rgba8,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::{Progress, Stage},
            request::*,
        },
        pipeline::{
            process::ProcessRequest,
            processor::{Allocator, Boundary, Processor, ResizeRequest},
            progress::Callback,
            quantize::{IndexedMetadataRef, QuantizeBoundary, QuantizeRequest},
        },
    },
    spec::resize::{common::alignment::ResizeAnchor, scalar::nearest::resize_nearest_into},
};

struct Io {
    input: Vec<u8>,
    sparse: bool,
    direct_output: bool,
    direct_completions: usize,
    gathers: usize,
    gather_offset_bytes: usize,
    snapshots: Vec<bool>,
    fail_gather: bool,
    fail_complete: bool,
    fail_stage: Option<Stage>,
    time: u64,
}

impl Io {
    fn new(width: u32, height: u32) -> Self {
        Self {
            input: (0..width * height * 4)
                .map(|i| (i.wrapping_mul(73) + i / 251) as u8)
                .collect(),
            sparse: true,
            direct_output: false,
            direct_completions: 0,
            gathers: 0,
            gather_offset_bytes: 0,
            snapshots: Vec::new(),
            fail_gather: false,
            fail_complete: false,
            fail_stage: None,
            time: 0,
        }
    }
}

impl Callback for Io {
    fn now_ms(&mut self) -> Result<u64, Failure> {
        self.time += 100;
        Ok(self.time)
    }

    fn report(&mut self, progress: Progress) -> Result<(), ()> {
        if Some(progress.stage) == self.fail_stage {
            Err(())
        } else {
            Ok(())
        }
    }
}

impl Boundary for Io {
    type Output = Vec<u8>;

    fn progress(&mut self) -> Option<&mut dyn Callback> {
        self.fail_stage.map(|_| self as &mut dyn Callback)
    }

    fn input_len(&mut self) -> Result<usize, Failure> {
        Ok(self.input.len())
    }

    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        destination.copy_from_slice(&self.input);
        Ok(())
    }

    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        self.snapshots.push(compare);
        if compare && destination == self.input {
            return Ok(true);
        }
        Boundary::copy_input(self, destination)?;
        Ok(false)
    }

    fn supports_sparse_input(&self) -> bool {
        self.sparse
    }

    fn supports_sparse_output(&self) -> bool {
        self.direct_output
    }

    fn complete_sparse(
        &mut self,
        columns: &[u8],
        rows: &[u8],
        source_len: usize,
        dimensions: ImageDimensions,
    ) -> Result<Vec<u8>, Failure> {
        self.direct_completions += 1;
        let mut bytes = vec![0; dimensions.storage_len::<Rgba8>().unwrap()];
        Boundary::gather_input(self, &mut bytes, columns, rows, source_len)?;
        Boundary::complete(self, &bytes, dimensions)
    }

    fn gather_input(
        &mut self,
        destination: &mut [u8],
        columns: &[u8],
        rows: &[u8],
        source_len: usize,
    ) -> Result<(), Failure> {
        self.gathers += 1;
        assert_eq!(source_len, self.input.len());
        assert_eq!(destination.len(), columns.len() * rows.len() / 4);
        self.gather_offset_bytes = columns.len() + rows.len();
        // Fail after writing, so stale partial output cannot survive a failed call.
        for (destination, row) in destination
            .chunks_exact_mut(columns.len())
            .zip(rows.chunks_exact(4))
        {
            let row = u32::from_le_bytes(row.try_into().unwrap()) as usize;
            for (pixel, column) in destination.chunks_exact_mut(4).zip(columns.chunks_exact(4)) {
                let start = row + u32::from_le_bytes(column.try_into().unwrap()) as usize;
                pixel.copy_from_slice(&self.input[start..start + 4]);
                if self.fail_gather {
                    return Err(Failure::new(
                        ErrorCode::WasmMemoryUnavailable,
                        ErrorPath::SourceData,
                    ));
                }
            }
        }
        Ok(())
    }

    fn complete(&mut self, bytes: &[u8], _: ImageDimensions) -> Result<Vec<u8>, Failure> {
        if self.fail_complete {
            return Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Output,
            ));
        }
        Ok(bytes.to_vec())
    }
}

impl QuantizeBoundary for Io {
    type Output = IndexedImage;

    fn progress(&mut self) -> Option<&mut dyn Callback> {
        Boundary::progress(self)
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        Boundary::input_len(self)
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        Boundary::copy_input(self, destination)
    }
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        Boundary::snapshot_input(self, destination, compare)
    }
    fn supports_sparse_input(&self) -> bool {
        self.sparse
    }
    fn gather_input(
        &mut self,
        destination: &mut [u8],
        columns: &[u8],
        rows: &[u8],
        source_len: usize,
    ) -> Result<(), Failure> {
        Boundary::gather_input(self, destination, columns, rows, source_len)
    }
    fn complete(
        &mut self,
        bytes: &[u8],
        dimensions: ImageDimensions,
        metadata: IndexedMetadataRef<'_>,
    ) -> Result<IndexedImage, Failure> {
        let indices = Boundary::complete(self, bytes, dimensions)?;
        Ok(IndexedImage {
            indices: ImageBuf::<PaletteIndex8>::from_vec_packed(indices, dimensions).unwrap(),
            palette: metadata.palette.clone(),
            warnings: metadata.warnings.to_vec(),
        })
    }
}

const PALETTE: [PaletteEntry; 4] = [
    PaletteEntry::Color { rgb: [0; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Color { rgb: [255; 3] },
    PaletteEntry::Transparent {},
];

fn process_request(resize: ResizeRequest, dither: DitherPolicy) -> ProcessRequest<'static> {
    ProcessRequest {
        source_width: resize.source_width,
        source_height: resize.source_height,
        palette: &PALETTE,
        recipe: RecipeV1 {
            version: 1,
            output: resize.output,
            alpha: AlphaPolicy::Preserve { threshold: 128.0 },
            matching: MatchPolicy::SrgbEuclidean,
            dither,
        },
    }
}

fn dithers() -> [DitherPolicy; 5] {
    [
        DitherPolicy::None {},
        DitherPolicy::Separable {
            perturb: PerturbPolicy {
                field: Field::BlueNoise {},
                space: WorkingSpace::Oklab,
                strength: 0.7,
                placement: Placement::Adaptive {
                    radius: 1,
                    threshold: 0.3,
                    softness: 0.2,
                },
            },
        },
        DitherPolicy::Diffusion {
            kernel: Diffusion::SierraLite,
            feedback: DiffusionFeedback::Matching,
            strength: 0.7,
            serpentine: true,
            placement: Placement::Everywhere {},
        },
        DitherPolicy::Diffusion {
            kernel: Diffusion::FloydSteinberg,
            feedback: DiffusionFeedback::SrgbBytes,
            strength: 1.0,
            serpentine: true,
            placement: Placement::Everywhere {},
        },
        DitherPolicy::Yliluoma {
            size: BayerSize::Four,
            placement: Placement::Everywhere {},
        },
    ]
}

fn staged(input: &[u8], request: ProcessRequest<'_>) -> IndexedImage {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    let mut io = Io::new(1, 1);
    io.input = input.to_vec();
    io.sparse = false;
    let resized = processor
        .resize(
            ResizeRequest {
                source_width: request.source_width,
                source_height: request.source_height,
                output: request.recipe.output,
            },
            &mut io,
        )
        .unwrap();
    io.input = resized;
    processor
        .dither_and_quantize(
            QuantizeRequest {
                source_width: request.recipe.output.width,
                source_height: request.recipe.output.height,
                palette: request.palette,
                alpha: request.recipe.alpha,
                matching: request.recipe.matching,
            },
            request.recipe.dither,
            &mut io,
        )
        .unwrap()
}

#[test]
fn sparse_process_matches_staged_calls_for_all_modes_and_observes_mutations() {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    for (source, output) in [
        ((43, 37), (7, 5)),
        ((256, 256), (64, 64)),
        ((64, 64), (32, 32)),
    ] {
        let mut io = Io::new(source.0, source.1);
        for dither in dithers() {
            let request = process_request(request(source, output, Anchor::BottomRight), dither);
            let durable = processor.process(request, &mut io).unwrap();
            assert_eq!(durable, staged(&io.input, request));
            io.input.fill(0);
            let transparent = processor.process(request, &mut io).unwrap();
            assert_eq!(transparent, staged(&io.input, request));
            io.input.fill(255);
            let changed = processor.process(request, &mut io).unwrap();
            assert_eq!(changed, staged(&io.input, request));
            assert_ne!(transparent, changed);
        }
        assert!(io.snapshots.is_empty());
        assert_eq!(io.gathers, 15);
    }
}

#[test]
fn sparse_process_full_transitions_never_reuse_offset_bytes_as_a_snapshot() {
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    let mut io = Io::new(64, 64);
    let full = process_request(
        request((64, 64), (48, 48), Anchor::TopLeft),
        DitherPolicy::None {},
    );
    let sparse = process_request(
        request((64, 64), (4, 4), Anchor::TopLeft),
        DitherPolicy::None {},
    );
    processor.process(full, &mut io).unwrap();
    processor.process(full, &mut io).unwrap();
    processor.process(sparse, &mut io).unwrap();
    io.input[0..4].fill(255);
    assert_eq!(
        processor.process(sparse, &mut io).unwrap(),
        staged(&io.input, sparse)
    );
    assert_eq!(
        processor.process(full, &mut io).unwrap(),
        staged(&io.input, full)
    );
    assert_eq!(io.snapshots, [false, true, false]);
    io.input[4] ^= 255; // Unsampled data changes without a full-source snapshot.
    assert_eq!(
        processor.process(sparse, &mut io).unwrap(),
        staged(&io.input, sparse)
    );
    io.sparse = false;
    assert_eq!(
        processor.process(sparse, &mut io).unwrap(),
        staged(&io.input, sparse)
    );
    assert_eq!(io.gathers, 3);
}

#[test]
fn sparse_process_budget_and_partial_gather_failures_recover_before_publication() {
    let request = process_request(request((256, 256), (4, 4), Anchor::Center), dithers()[1]);
    let mut io = Io::new(256, 256);
    let mut probe = Processor::new(1 << 20, 0).unwrap();
    let expected = probe.process(request, &mut io).unwrap();
    let limit = budget_support::minimum(probe.peak_capacity_bytes(), |limit| {
        Processor::new(limit, 0)
            .and_then(|mut processor| processor.process(request, &mut Io::new(256, 256)))
            .is_ok()
    });
    assert!(limit < io.input.len() as u64);
    let mut exact = Processor::new(limit, 0).unwrap();
    assert_eq!(exact.process(request, &mut io).unwrap(), expected);
    let before = io.gathers;
    let mut short = Processor::new(limit - 1, 0).unwrap();
    assert_eq!(
        short.process(request, &mut io).unwrap_err().code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(io.gathers, before);
    struct Reservation {
        calls: usize,
        fail_at: usize,
        extra: usize,
    }
    impl Allocator for Reservation {
        fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure> {
            self.calls += 1;
            if self.calls == self.fail_at {
                return Err(Failure::new(
                    ErrorCode::WasmMemoryUnavailable,
                    ErrorPath::Wasm,
                ));
            }
            buffer.try_reserve_exact(additional + self.extra).unwrap();
            Ok(())
        }
    }
    for fail_at in 0..=4 {
        let mut processor = Processor::new(limit, 0).unwrap();
        let mut reservation = Reservation {
            calls: 0,
            fail_at,
            extra: usize::from(fail_at == 0),
        };
        assert_eq!(
            processor
                .process_with_allocator(request, &mut io, &mut reservation)
                .unwrap_err()
                .code,
            if fail_at == 0 {
                ErrorCode::MemoryLimit
            } else {
                ErrorCode::WasmMemoryUnavailable
            }
        );
        assert_eq!(io.gathers, before);
    }
    for phase in 0..6 {
        io.fail_gather = phase == 0;
        io.fail_complete = phase == 1;
        io.fail_stage = match phase {
            2 => Some(Stage::Resize),
            3 => Some(Stage::Perturb),
            4 => Some(Stage::Quantize),
            5 => Some(Stage::Complete),
            _ => None,
        };
        assert_eq!(
            exact.process(request, &mut io).unwrap_err().code,
            if phase < 2 {
                ErrorCode::WasmMemoryUnavailable
            } else {
                ErrorCode::Callback
            }
        );
        io.fail_gather = false;
        io.fail_complete = false;
        io.fail_stage = None;
        assert_eq!(exact.process(request, &mut io).unwrap(), expected);
    }
    exact.dispose().unwrap();
    assert_eq!(
        exact.process(request, &mut io).unwrap_err().code,
        ErrorCode::Disposed
    );
}

#[path = "support/budget.rs"]
mod budget_support;

#[test]
fn sparse_process_drops_a_large_idle_snapshot_before_reserving_offset_scratch() {
    let mut io = Io::new(256, 256);
    let limit = Processor::bookkeeping_bytes(0) + io.input.len() as u64 + 4096;
    let mut processor = Processor::new(limit, 0).unwrap();
    processor
        .resize(request((256, 256), (256, 256), Anchor::Center), &mut io)
        .unwrap();
    let request = process_request(
        request((256, 256), (64, 64), Anchor::Center),
        DitherPolicy::None {},
    );
    assert_eq!(
        processor.process(request, &mut io).unwrap(),
        staged(&io.input, request)
    );
    assert!(processor.peak_capacity_bytes() <= limit);
}

fn request(source: (u32, u32), output: (u32, u32), anchor: Anchor) -> ResizeRequest {
    ResizeRequest {
        source_width: source.0,
        source_height: source.1,
        output: Output {
            width: output.0,
            height: output.1,
            resize: ResizePolicy::Nearest { anchor },
        },
    }
}

fn expected(input: &[u8], request: ResizeRequest, anchor: ResizeAnchor) -> Vec<u8> {
    let source = ImageDimensions::new(request.source_width, request.source_height).unwrap();
    let output = ImageDimensions::new(request.output.width, request.output.height).unwrap();
    let mut bytes = vec![0; output.storage_len::<Rgba8>().unwrap()];
    resize_nearest_into(
        ImageView::<Rgba8>::packed(input, source).unwrap(),
        ImageViewMut::packed(&mut bytes, output).unwrap(),
        anchor,
    );
    bytes
}

#[test]
fn sparse_samples_match_frozen_nearest_for_every_anchor_and_scale_plan() {
    for (source, output) in [
        ((40, 32), (20, 16)), // Exact factors at the candidate cutoff.
        ((41, 33), (20, 16)), // Fractional maps just below the cutoff.
        ((40, 32), (10, 8)),  // Quarter-sized dimensions.
        ((45, 63), (5, 7)),   // Odd exact factors.
        ((43, 37), (7, 5)),   // Fractional maps.
        ((2, 128), (4, 2)),   // Mixed upscale and downscale axes.
        ((128, 3), (4, 3)),   // Same-height map.
        ((3, 128), (3, 4)),   // Same-width map.
        ((64, 64), (1, 17)),  // One output column.
        ((64, 64), (17, 1)),  // One output row.
        ((64, 64), (1, 1)),   // One sampled pixel.
    ] {
        let mut processor = Processor::new(1_000_000, 0).unwrap();
        let mut io = Io::new(source.0, source.1);
        for (anchor, oracle) in [
            (Anchor::TopLeft, ResizeAnchor::TopLeft),
            (Anchor::Top, ResizeAnchor::Top),
            (Anchor::TopRight, ResizeAnchor::TopRight),
            (Anchor::Left, ResizeAnchor::Left),
            (Anchor::Center, ResizeAnchor::Center),
            (Anchor::Right, ResizeAnchor::Right),
            (Anchor::BottomLeft, ResizeAnchor::BottomLeft),
            (Anchor::Bottom, ResizeAnchor::Bottom),
            (Anchor::BottomRight, ResizeAnchor::BottomRight),
        ] {
            let request = request(source, output, anchor);
            assert_eq!(
                processor.resize(request, &mut io).unwrap(),
                expected(&io.input, request, oracle)
            );
        }
        assert_eq!(io.gathers, 9);
        assert_eq!(io.gather_offset_bytes, (output.0 + output.1) as usize * 4);
        assert!(io.snapshots.is_empty());
    }
}

#[test]
fn sparse_calls_observe_mutations_and_invalidate_full_source_identity() {
    let mut processor = Processor::new(1_000_000, 0).unwrap();
    let mut io = Io::new(64, 64);
    let sparse = request((64, 64), (4, 4), Anchor::TopLeft);
    let full = request((64, 64), (48, 48), Anchor::TopLeft);
    processor.resize(full, &mut io).unwrap();
    processor.resize(full, &mut io).unwrap();
    assert_eq!(io.snapshots, [false, true]);
    let durable = processor.resize(sparse, &mut io).unwrap();
    io.input[0] ^= 255;
    let changed = processor.resize(sparse, &mut io).unwrap();
    assert_ne!(durable, changed);
    io.input[4] ^= 255; // An unsampled pixel changes; gather still happens.
    assert_eq!(processor.resize(sparse, &mut io).unwrap(), changed);
    assert_eq!(io.gathers, 3);
    assert_eq!(
        processor.resize(full, &mut io).unwrap(),
        expected(&io.input, full, ResizeAnchor::TopLeft)
    );
    assert_eq!(io.snapshots, [false, true, false]);
    io.sparse = false;
    assert_eq!(processor.resize(sparse, &mut io).unwrap(), changed);
    assert_eq!(io.gathers, 3);
    processor.dispose().unwrap();
    assert_eq!(
        processor.resize(sparse, &mut io).unwrap_err().code,
        ErrorCode::Disposed
    );
    assert_ne!(durable, changed);
}

#[test]
fn sparse_capacity_preflight_and_actual_reservations_precede_gather() {
    let request = request((256, 256), (8, 8), Anchor::Center);
    let mut io = Io::new(256, 256);
    let mut probe = Processor::new(1_000_000, 0).unwrap();
    probe.resize(request, &mut io).unwrap();
    let limit = probe.peak_capacity_bytes();
    assert!(limit < io.input.len() as u64);
    let mut exact = Processor::new(limit, 0).unwrap();
    assert!(exact.resize(request, &mut io).is_ok());
    let before = io.gathers;
    let mut short = Processor::new(limit - 1, 0).unwrap();
    assert_eq!(
        short.resize(request, &mut io).unwrap_err().code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(io.gathers, before);

    struct Excess;
    impl Allocator for Excess {
        fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure> {
            buffer.try_reserve_exact(additional + 1).unwrap();
            Ok(())
        }
    }
    let mut excess = Processor::new(limit, 0).unwrap();
    assert_eq!(
        excess
            .resize_with_allocator(request, &mut io, &mut Excess)
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(io.gathers, before);

    struct Fail;
    impl Allocator for Fail {
        fn reserve(&mut self, _: &mut Vec<u8>, _: usize) -> Result<(), Failure> {
            Err(Failure::new(
                ErrorCode::WasmMemoryUnavailable,
                ErrorPath::Wasm,
            ))
        }
    }
    let mut failed = Processor::new(limit, 0).unwrap();
    assert_eq!(
        failed
            .resize_with_allocator(request, &mut io, &mut Fail)
            .unwrap_err()
            .code,
        ErrorCode::WasmMemoryUnavailable
    );
    assert_eq!(io.gathers, before);
    assert!(failed.resize(request, &mut io).is_ok());
}

#[test]
fn sparse_copy_callback_and_validation_failures_recover() {
    let request = request((64, 64), (4, 4), Anchor::Center);
    let mut processor = Processor::new(1_000_000, 0).unwrap();
    let mut io = Io::new(64, 64);
    let oracle = expected(&io.input, request, ResizeAnchor::Center);
    for phase in 0..5 {
        io.fail_gather = phase == 0;
        io.fail_complete = phase == 1;
        io.fail_stage = match phase {
            2 => Some(Stage::Prepare),
            3 => Some(Stage::Resize),
            4 => Some(Stage::Complete),
            _ => None,
        };
        assert_eq!(
            processor.resize(request, &mut io).unwrap_err().code,
            if phase < 2 {
                ErrorCode::WasmMemoryUnavailable
            } else {
                ErrorCode::Callback
            }
        );
        io.fail_gather = false;
        io.fail_complete = false;
        io.fail_stage = None;
        assert_eq!(processor.resize(request, &mut io).unwrap(), oracle);
    }
    io.input.pop();
    let before = io.gathers;
    assert_eq!(
        processor.resize(request, &mut io).unwrap_err(),
        Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData)
    );
    assert_eq!(io.gathers, before);
}

#[test]
fn direct_sparse_output_charges_pixels_but_reserves_only_offsets() {
    let request = request((64, 64), (4, 4), Anchor::Center);
    let mut io = Io::new(64, 64);
    let mut fallback = Processor::new(1 << 20, 0).unwrap();
    let expected = fallback.resize(request, &mut io).unwrap();
    let limit = fallback.peak_capacity_bytes();
    io.direct_output = true;
    struct Reservations {
        sizes: Vec<usize>,
        extra: usize,
    }
    impl Allocator for Reservations {
        fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure> {
            self.sizes.push(additional);
            buffer.try_reserve_exact(additional + self.extra).unwrap();
            Ok(())
        }
    }
    let mut reservations = Reservations {
        sizes: Vec::new(),
        extra: 0,
    };
    let mut direct = Processor::new(limit, 0).unwrap();
    assert_eq!(
        direct
            .resize_with_allocator(request, &mut io, &mut reservations)
            .unwrap(),
        expected
    );
    assert_eq!(
        reservations.sizes,
        [32],
        "only four column and four row offsets enter Wasm"
    );
    assert_eq!(
        direct.peak_capacity_bytes(),
        limit,
        "64 output bytes remain charged"
    );
    let completions = io.direct_completions;
    let mut short = Processor::new(limit - 1, 0).unwrap();
    assert_eq!(
        short.resize(request, &mut io).unwrap_err().code,
        ErrorCode::MemoryLimit
    );
    let mut excess = Processor::new(limit, 0).unwrap();
    reservations.extra = 1;
    assert_eq!(
        excess
            .resize_with_allocator(request, &mut io, &mut reservations)
            .unwrap_err()
            .code,
        ErrorCode::MemoryLimit
    );
    assert_eq!(
        io.direct_completions, completions,
        "preflight rejects before constructing output"
    );
}

#[test]
fn direct_sparse_output_recovers_from_boundary_failures_and_keeps_callback_ordering() {
    let request = request((64, 64), (4, 4), Anchor::BottomRight);
    let mut processor = Processor::new(1 << 20, 0).unwrap();
    let mut io = Io::new(64, 64);
    io.direct_output = true;
    let durable = processor.resize(request, &mut io).unwrap();
    for phase in 0..5 {
        io.fail_gather = phase == 0;
        io.fail_complete = phase == 1;
        io.fail_stage = match phase {
            2 => Some(Stage::Prepare),
            3 => Some(Stage::Resize),
            4 => Some(Stage::Complete),
            _ => None,
        };
        let completions = io.direct_completions;
        assert_eq!(
            processor.resize(request, &mut io).unwrap_err().code,
            if phase < 2 {
                ErrorCode::WasmMemoryUnavailable
            } else {
                ErrorCode::Callback
            }
        );
        assert_eq!(io.direct_completions, completions + usize::from(phase < 2));
        io.fail_gather = false;
        io.fail_complete = false;
        io.fail_stage = None;
        assert_eq!(processor.resize(request, &mut io).unwrap(), durable);
    }
    processor.dispose().unwrap();
    assert_eq!(
        processor.resize(request, &mut io).unwrap_err().code,
        ErrorCode::Disposed
    );
}
