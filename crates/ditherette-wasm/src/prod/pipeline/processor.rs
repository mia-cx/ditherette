//! Private nearest-only ownership around the landed packed kernel.
//!
//! Request, initialization, and lifecycle rules follow the copied contract.
//! Failures use static codes/paths instead of allocating diagnostic strings.

use std::mem::size_of;

use crate::image::{ImageDimensions, ImageView, ImageViewMut, Rgba8};

use crate::prod::{
    contract::{
        error::ErrorCode,
        failure::{ErrorPath, Failure},
        request::{
            Anchor, Output, ResizePolicy, MAX_MEMORY_LIMIT_BYTES, MAX_OUTPUT_SIDE, MAX_PIXELS,
            MAX_SOURCE_SIDE,
        },
    },
    resize::scalar::nearest::{
        alignment::ResizeAnchor, resize_nearest_rgba8_with_plan_into, NearestResizePlan,
        PlanAllocationError,
    },
};

/// Typed private shape. The package validates recipe version and raw property types.
#[derive(Debug, Clone, Copy)]
pub struct NearestRequest {
    pub source_width: u32,
    pub source_height: u32,
    pub output: Output,
}

/// Fallible capacity reservation. The production implementation uses try_reserve_exact.
pub trait Allocator {
    fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure>;
}

pub struct SystemAllocator;

impl Allocator for SystemAllocator {
    fn reserve(&mut self, buffer: &mut Vec<u8>, additional: usize) -> Result<(), Failure> {
        buffer
            .try_reserve_exact(additional)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))
    }
}

/// Borrowed input and durable output boundary. Every JavaScript call is caught by the adapter.
pub trait Boundary {
    type Output;
    fn input_len(&mut self) -> Result<usize, Failure>;
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure>;
    /// Constructs the complete durable result. Nothing is published if this fails.
    fn complete(
        &mut self,
        bytes: &[u8],
        dimensions: ImageDimensions,
    ) -> Result<Self::Output, Failure>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ready,
    Running,
    Disposed,
}

/// Per-instance control and budget. S19 retains no image or scratch allocations.
#[derive(Debug)]
pub struct Processor {
    memory_limit: u64,
    boundary_capacity: u64,
    peak_capacity: u64,
    state: State,
}

#[derive(Default)]
struct Buffers {
    source: Vec<u8>,
    output: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Plan {
    source: ImageDimensions,
    output: ImageDimensions,
    source_len: usize,
    output_len: usize,
    anchor: ResizeAnchor,
}

impl Processor {
    /// Counts owned control, buffer headers, and plan records, plus adapter-owned capacity.
    /// Compiler stack frames and fixed module overhead are outside this ownership accounting.
    pub const fn bookkeeping_bytes(boundary_capacity: u64) -> u64 {
        (size_of::<Self>()
            + size_of::<Buffers>()
            + size_of::<Plan>()
            + size_of::<Option<NearestResizePlan>>()) as u64
            + boundary_capacity
    }

    /// Checks the budget before the adapter primes any boundary allocation.
    pub fn new(memory_limit: u64, boundary_capacity: u64) -> Result<Self, Failure> {
        if memory_limit == 0 || memory_limit > MAX_MEMORY_LIMIT_BYTES {
            return Err(Failure::new(
                ErrorCode::InvalidSettings,
                ErrorPath::MemoryLimitBytes,
            ));
        }
        let overhead = Self::bookkeeping_bytes(boundary_capacity);
        if overhead > memory_limit {
            return Err(memory_limit_failure());
        }
        Ok(Self {
            memory_limit,
            boundary_capacity,
            peak_capacity: overhead,
            state: State::Ready,
        })
    }

    /// Private accounting observation for conformance and benchmark adapters.
    pub const fn peak_capacity_bytes(&self) -> u64 {
        self.peak_capacity
    }

    /// Releases retained ownership idempotently. There are no retained images in this slice.
    pub fn dispose(&mut self) -> Result<(), Failure> {
        if self.state == State::Running {
            return Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance));
        }
        self.state = State::Disposed;
        Ok(())
    }

    pub fn resize<B: Boundary>(
        &mut self,
        request: NearestRequest,
        boundary: &mut B,
    ) -> Result<B::Output, Failure> {
        self.resize_with_allocator(request, boundary, &mut SystemAllocator)
    }

    /// Runs a call with injectable reservation failures for independent ownership fixtures.
    pub fn resize_with_allocator<B: Boundary, A: Allocator>(
        &mut self,
        request: NearestRequest,
        boundary: &mut B,
        allocator: &mut A,
    ) -> Result<B::Output, Failure> {
        match self.state {
            State::Disposed => return Err(Failure::new(ErrorCode::Disposed, ErrorPath::Instance)),
            State::Running => {
                return Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance))
            }
            State::Ready => {}
        }
        self.state = State::Running;
        let result = self.run(request, boundary, allocator);
        self.state = State::Ready;
        result
    }

    fn run<B: Boundary, A: Allocator>(
        &mut self,
        request: NearestRequest,
        boundary: &mut B,
        allocator: &mut A,
    ) -> Result<B::Output, Failure> {
        let plan = Plan::new(request, boundary.input_len()?)?;
        let overhead = Self::bookkeeping_bytes(self.boundary_capacity);
        self.peak_capacity = overhead;
        let metadata_bytes = if plan.source == plan.output {
            0
        } else {
            NearestResizePlan::required_capacity_bytes(plan.source, plan.output)
        };
        let planned = overhead
            .checked_add(plan.source_len as u64)
            .and_then(|bytes| bytes.checked_add(plan.output_len as u64))
            .and_then(|bytes| bytes.checked_add(metadata_bytes))
            .ok_or_else(memory_limit_failure)?;
        if planned > self.memory_limit {
            return Err(memory_limit_failure());
        }

        let metadata = if plan.source == plan.output {
            None
        } else {
            let budget =
                self.memory_limit - overhead - plan.source_len as u64 - plan.output_len as u64;
            Some(
                NearestResizePlan::try_new(plan.source, plan.output, plan.anchor, budget).map_err(
                    |error| match error {
                        PlanAllocationError::MemoryLimit => memory_limit_failure(),
                        PlanAllocationError::Allocation => {
                            Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
                        }
                    },
                )?,
            )
        };
        let overhead = overhead
            + metadata
                .as_ref()
                .map_or(0, NearestResizePlan::capacity_bytes);
        self.peak_capacity = overhead;
        let mut buffers = Buffers::default();
        allocator.reserve(&mut buffers.source, plan.source_len)?;
        self.check_capacity(&buffers, overhead, plan.output_len)?;
        allocator.reserve(&mut buffers.output, plan.output_len)?;
        self.check_capacity(&buffers, overhead, 0)?;
        // Both reservations finish before either resize can grow a Vec.
        buffers.source.resize(plan.source_len, 0);
        buffers.output.resize(plan.output_len, 0);
        boundary.copy_input(&mut buffers.source)?;
        let source = ImageView::<Rgba8>::packed(&buffers.source, plan.source)
            .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
        let output = ImageViewMut::<Rgba8>::packed(&mut buffers.output, plan.output)
            .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
        if let Some(metadata) = &metadata {
            resize_nearest_rgba8_with_plan_into(source, output, metadata);
        } else {
            // The landed convenience entrypoint also bypasses planning for identity.
            buffers.output.copy_from_slice(&buffers.source);
        }
        // A failed complete helper drops both Vecs. Future callback/cache publication
        // belongs after this complete result exists, never before it.
        boundary.complete(&buffers.output, plan.output)
    }

    fn check_capacity(
        &mut self,
        buffers: &Buffers,
        overhead: u64,
        remaining_output: usize,
    ) -> Result<(), Failure> {
        let actual = overhead + buffers.source.capacity() as u64 + buffers.output.capacity() as u64;
        self.peak_capacity = self.peak_capacity.max(actual);
        if actual + remaining_output as u64 > self.memory_limit {
            return Err(memory_limit_failure());
        }
        Ok(())
    }
}

impl Plan {
    fn new(request: NearestRequest, input_len: usize) -> Result<Self, Failure> {
        let ResizePolicy::Nearest { anchor } = request.output.resize else {
            return Err(Failure::new(
                ErrorCode::UnsupportedOperation,
                ErrorPath::OutputResize,
            ));
        };
        let source = dimensions(request.source_width, request.source_height, true)?;
        let source_len = source
            .storage_len::<Rgba8>()
            .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
        if input_len != source_len {
            return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
        }
        let output = dimensions(request.output.width, request.output.height, false)?;
        let output_len = output
            .storage_len::<Rgba8>()
            .map_err(|_| Failure::new(ErrorCode::InvalidSettings, ErrorPath::Output))?;
        let anchor = match anchor {
            Anchor::TopLeft => ResizeAnchor::TopLeft,
            Anchor::Top => ResizeAnchor::Top,
            Anchor::TopRight => ResizeAnchor::TopRight,
            Anchor::Left => ResizeAnchor::Left,
            Anchor::Center => ResizeAnchor::Center,
            Anchor::Right => ResizeAnchor::Right,
            Anchor::BottomLeft => ResizeAnchor::BottomLeft,
            Anchor::Bottom => ResizeAnchor::Bottom,
            Anchor::BottomRight => ResizeAnchor::BottomRight,
        };
        Ok(Self {
            source,
            output,
            source_len,
            output_len,
            anchor,
        })
    }
}

fn dimensions(width: u32, height: u32, source: bool) -> Result<ImageDimensions, Failure> {
    let (limit, code, width_path, height_path, image_path) = if source {
        (
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceWidth,
            ErrorPath::SourceHeight,
            ErrorPath::Source,
        )
    } else {
        (
            MAX_OUTPUT_SIDE,
            ErrorCode::InvalidSettings,
            ErrorPath::OutputWidth,
            ErrorPath::OutputHeight,
            ErrorPath::Output,
        )
    };
    if width == 0 || width > limit {
        return Err(Failure::new(code, width_path));
    }
    if height == 0 || height > limit {
        return Err(Failure::new(code, height_path));
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Failure::new(code, image_path));
    }
    ImageDimensions::new(width, height).map_err(|_| Failure::new(code, image_path))
}

fn memory_limit_failure() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
