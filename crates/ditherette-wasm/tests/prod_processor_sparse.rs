use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::{Progress, Stage},
            request::{Anchor, Output, ResizePolicy},
        },
        pipeline::{
            processor::{Allocator, Boundary, Processor, ResizeRequest},
            progress::Callback,
        },
    },
    spec::resize::{common::alignment::ResizeAnchor, scalar::nearest::resize_nearest_into},
};

struct Io {
    input: Vec<u8>,
    sparse: bool,
    gathers: usize,
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
            gathers: 0,
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
        self.copy_input(destination)?;
        Ok(false)
    }

    fn supports_sparse_input(&self) -> bool {
        self.sparse
    }

    fn gather_input(
        &mut self,
        destination: &mut [u8],
        offsets: &[u8],
        source_len: usize,
    ) -> Result<(), Failure> {
        self.gathers += 1;
        assert_eq!(source_len, self.input.len());
        assert_eq!(destination.len(), offsets.len());
        // Fail after writing, so stale partial output cannot survive a failed call.
        for (pixel, offset) in destination.chunks_exact_mut(4).zip(offsets.chunks_exact(4)) {
            let start = u32::from_le_bytes(offset.try_into().unwrap()) as usize;
            pixel.copy_from_slice(&self.input[start..start + 4]);
            if self.fail_gather {
                return Err(Failure::new(
                    ErrorCode::WasmMemoryUnavailable,
                    ErrorPath::SourceData,
                ));
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
        ((40, 32), (10, 8)), // Exact factors at the cutoff.
        ((45, 63), (5, 7)),  // Odd exact factors.
        ((43, 37), (7, 5)),  // Fractional maps.
        ((2, 128), (4, 2)),  // Mixed upscale and downscale axes.
        ((128, 3), (4, 3)),  // Same-height map.
        ((3, 128), (3, 4)),  // Same-width map.
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
        assert!(io.snapshots.is_empty());
    }
}

#[test]
fn sparse_calls_observe_mutations_and_invalidate_full_source_identity() {
    let mut processor = Processor::new(1_000_000, 0).unwrap();
    let mut io = Io::new(64, 64);
    let sparse = request((64, 64), (4, 4), Anchor::TopLeft);
    let full = request((64, 64), (32, 32), Anchor::TopLeft);
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
