//! Private resize ownership around landed packed kernels and fallible preparation.
//!
//! Request, initialization, and lifecycle rules follow the copied contract.
//! Failures use static codes/paths instead of allocating diagnostic strings.

use std::mem::size_of;

use crate::image::{ImageDimensions, ImageView, ImageViewMut, Rgba8};

use super::resize::{self, PreparedResize};
use crate::prod::contract::{
    error::ErrorCode,
    failure::{ErrorPath, Failure},
    lifecycle::Stage,
    request::{
        Output, ResizePolicy, MAX_MEMORY_LIMIT_BYTES, MAX_OUTPUT_SIDE, MAX_PIXELS, MAX_SOURCE_SIDE,
    },
};

/// Typed private shape. The package validates recipe version and raw property types.
#[derive(Debug, Clone, Copy)]
pub struct ResizeRequest {
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
    /// Borrow this call's optional caught callback without allocating a handle.
    fn progress(&mut self) -> Option<&mut dyn super::progress::Callback> {
        None
    }
    fn input_len(&mut self) -> Result<usize, Failure>;
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure>;
    /// Return true only after exact equality with the current input; otherwise copy it.
    /// Boundaries without comparison support always copy and request a fresh identity.
    fn snapshot_input(&mut self, destination: &mut [u8], _compare: bool) -> Result<bool, Failure> {
        self.copy_input(destination)?;
        Ok(false)
    }
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

/// Per-instance control, shared preparation/image reuse, and capacity-accounted idle scratch.
#[derive(Debug)]
pub struct Processor {
    memory_limit: u64,
    boundary_capacity: u64,
    peak_capacity: u64,
    state: State,
    preparation: super::preparation::Store,
}

#[derive(Clone, Copy)]
struct Plan {
    source: ImageDimensions,
    output: ImageDimensions,
    source_len: usize,
    output_len: usize,
    resize: ResizePolicy,
}

impl Processor {
    /// Observe the private candidate without changing other stage selections.
    #[cfg(any(test, feature = "bench-subjects"))]
    pub fn execution_policy(&self) -> super::execution::ExecutionPolicy {
        self.preparation.execution_policy()
    }

    /// Development-only scheduling override. Public package settings never expose execution policy.
    #[cfg(any(test, feature = "bench-subjects"))]
    pub fn set_execution_policy(
        &mut self,
        policy: super::execution::ExecutionPolicy,
    ) -> Result<(), Failure> {
        self.validate_execution_policy(policy)?;
        self.preparation.execution = policy;
        self.preparation.execution_overrides = 0b111;
        Ok(())
    }

    /// Override only this stage; None explicitly forces its scalar path.
    #[cfg(any(test, feature = "bench-subjects"))]
    pub fn set_execution_stage(
        &mut self,
        stage: super::execution::ExecutionStage,
        band: Option<super::execution::RowBandPolicy>,
    ) -> Result<(), Failure> {
        let mut policy = self.preparation.execution;
        policy.set_stage(stage, band);
        self.validate_execution_policy(policy)?;
        self.preparation.execution = policy;
        self.preparation.execution_overrides |= stage.mask();
        Ok(())
    }

    #[cfg(any(test, feature = "bench-subjects"))]
    fn validate_execution_policy(
        &self,
        policy: super::execution::ExecutionPolicy,
    ) -> Result<(), Failure> {
        match self.state {
            State::Disposed => return Err(Failure::new(ErrorCode::Disposed, ErrorPath::Instance)),
            State::Running => {
                return Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance))
            }
            State::Ready => {}
        }
        if [policy.resize, policy.indexed, policy.mixing]
            .into_iter()
            .flatten()
            .any(|band| band.height == 0)
        {
            return Err(Failure::new(ErrorCode::InvalidSettings, ErrorPath::Control));
        }
        Ok(())
    }

    /// Counts owned control, buffer headers, and plan records, plus adapter-owned capacity.
    /// Compiler stack frames and fixed module overhead are outside this ownership accounting.
    pub const fn bookkeeping_bytes(boundary_capacity: u64) -> u64 {
        (size_of::<Self>() + size_of::<Plan>()) as u64
            + super::preparation::Call::record_bytes()
            + size_of::<super::progress::Control>() as u64
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
            preparation: super::preparation::Store::default(),
        })
    }

    /// Private accounting observation for conformance and benchmark adapters.
    pub const fn peak_capacity_bytes(&self) -> u64 {
        self.peak_capacity
    }

    /// Releases retained preparation, image stages, and scratch idempotently.
    pub fn dispose(&mut self) -> Result<(), Failure> {
        if self.state == State::Running {
            return Err(Failure::new(ErrorCode::ReentrantCall, ErrorPath::Instance));
        }
        self.preparation = super::preparation::Store::default();
        self.state = State::Disposed;
        Ok(())
    }

    pub fn resize<B: Boundary>(
        &mut self,
        request: ResizeRequest,
        boundary: &mut B,
    ) -> Result<B::Output, Failure> {
        self.resize_with_allocator(request, boundary, &mut SystemAllocator)
    }

    /// Resize then dither into one durable indexed result, retaining both RGBA8 barriers.
    pub fn process<B: super::quantize::QuantizeBoundary>(
        &mut self,
        request: super::process::ProcessRequest<'_>,
        boundary: &mut B,
    ) -> Result<B::Output, Failure> {
        self.process_with_allocator(request, boundary, &mut SystemAllocator)
    }

    /// Snapshot current input, then preflight hit-aware execution with recoverable failures.
    pub fn process_with_allocator<B: super::quantize::QuantizeBoundary, A: Allocator>(
        &mut self,
        request: super::process::ProcessRequest<'_>,
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
        // Call bookkeeping includes preparation handles and all four scratch Vec records.
        let overhead = Self::bookkeeping_bytes(self.boundary_capacity)
            + size_of::<super::process::ProcessRequest<'_>>() as u64
            + if matches!(
                request.recipe.dither,
                crate::prod::contract::request::DitherPolicy::Separable { .. }
            ) {
                super::perturb::working_capacity_bytes()
            } else if matches!(
                request.recipe.dither,
                crate::prod::contract::request::DitherPolicy::Diffusion { .. }
            ) {
                size_of::<crate::prod::dither::error_diffusion::prepared::DiffusionPolicy>() as u64
            } else {
                0
            }
            + boundary.capacity_bytes();
        self.peak_capacity = overhead;
        let result = super::process::run(
            request,
            boundary,
            allocator,
            self.memory_limit,
            overhead,
            &mut self.peak_capacity,
            &mut self.preparation,
        );
        self.state = State::Ready;
        result
    }

    /// Snapshot current input and materialize palette-free, durable RGBA8 within the budget.
    pub fn perturb<B: Boundary>(
        &mut self,
        request: super::perturb::PerturbRequest,
        boundary: &mut B,
    ) -> Result<B::Output, Failure> {
        self.perturb_with_allocator(request, boundary, &mut SystemAllocator)
    }

    /// Injectable reservations preserve the same recoverable lifecycle as resize and quantize.
    pub fn perturb_with_allocator<B: Boundary, A: Allocator>(
        &mut self,
        request: super::perturb::PerturbRequest,
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
        let overhead = Self::bookkeeping_bytes(self.boundary_capacity);
        self.peak_capacity = overhead;
        let result = super::perturb::run(
            request,
            boundary,
            allocator,
            self.memory_limit,
            overhead,
            &mut self.peak_capacity,
            &mut self.preparation,
        );
        self.state = State::Ready;
        result
    }

    /// Separable modes quantize a complete RGBA8 intermediate; diffusion uses three work rows.
    /// None delegates to direct quantization.
    /// Yliluoma searches literal ordered mixtures.
    pub fn dither_and_quantize<B: super::quantize::QuantizeBoundary>(
        &mut self,
        request: super::quantize::QuantizeRequest<'_>,
        dither: crate::prod::contract::request::DitherPolicy,
        boundary: &mut B,
    ) -> Result<B::Output, Failure> {
        self.dither_and_quantize_with_allocator(request, dither, boundary, &mut SystemAllocator)
    }

    /// Snapshot current input before reserving mode-specific work not supplied by image hits.
    pub fn dither_and_quantize_with_allocator<
        B: super::quantize::QuantizeBoundary,
        A: Allocator,
    >(
        &mut self,
        request: super::quantize::QuantizeRequest<'_>,
        dither: crate::prod::contract::request::DitherPolicy,
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
        use crate::prod::contract::request::DitherPolicy;
        if matches!(dither, DitherPolicy::None {}) {
            return self.quantize_with_allocator(request, boundary, allocator);
        }
        self.state = State::Running;
        let mode_capacity = if matches!(dither, DitherPolicy::Diffusion { .. }) {
            size_of::<crate::prod::dither::error_diffusion::prepared::DiffusionPolicy>() as u64
                + size_of::<DitherPolicy>() as u64
        } else if matches!(dither, DitherPolicy::Yliluoma { .. }) {
            size_of::<DitherPolicy>() as u64
        } else {
            (size_of::<crate::prod::contract::request::PerturbPolicy>() + size_of::<Vec<u8>>())
                as u64
        };
        let overhead = Self::bookkeeping_bytes(self.boundary_capacity)
            + size_of::<super::quantize::QuantizeRequest<'_>>() as u64
            + mode_capacity
            + if matches!(dither, DitherPolicy::Separable { .. }) {
                super::perturb::working_capacity_bytes()
            } else {
                0
            }
            + boundary.capacity_bytes();
        self.peak_capacity = overhead;
        let result = match dither {
            DitherPolicy::Diffusion { .. } => super::diffusion::run(
                request,
                dither,
                boundary,
                allocator,
                self.memory_limit,
                overhead,
                &mut self.peak_capacity,
                &mut self.preparation,
            ),
            DitherPolicy::Separable { .. } | DitherPolicy::Yliluoma { .. } => {
                super::quantize::run_with_dither(
                    request,
                    dither,
                    boundary,
                    allocator,
                    self.memory_limit,
                    overhead,
                    &mut self.peak_capacity,
                    &mut self.preparation,
                )
            }
            _ => unreachable!("supported family checked before entering running state"),
        };
        self.state = State::Ready;
        result
    }

    /// Quantize into durable indexed output after complete call-owned capacity preflight.
    pub fn quantize<B: super::quantize::QuantizeBoundary>(
        &mut self,
        request: super::quantize::QuantizeRequest<'_>,
        boundary: &mut B,
    ) -> Result<B::Output, Failure> {
        self.quantize_with_allocator(request, boundary, &mut SystemAllocator)
    }

    /// Injectable source/index reservations; palette preparation also reserves fallibly.
    pub fn quantize_with_allocator<B: super::quantize::QuantizeBoundary, A: Allocator>(
        &mut self,
        request: super::quantize::QuantizeRequest<'_>,
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
        let overhead = Self::bookkeeping_bytes(self.boundary_capacity)
            + size_of::<super::quantize::QuantizeRequest<'_>>() as u64
            + boundary.capacity_bytes();
        self.peak_capacity = overhead;
        let result = super::quantize::run(
            request,
            boundary,
            allocator,
            self.memory_limit,
            overhead,
            &mut self.peak_capacity,
            &mut self.preparation,
        );
        self.state = State::Ready;
        result
    }

    /// Runs a call with injectable reservation failures for independent ownership fixtures.
    pub fn resize_with_allocator<B: Boundary, A: Allocator>(
        &mut self,
        request: ResizeRequest,
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
        request: ResizeRequest,
        boundary: &mut B,
        allocator: &mut A,
    ) -> Result<B::Output, Failure> {
        let plan = Plan::new(request, boundary.input_len()?)?;
        let enabled = boundary.progress().is_some();
        let mut progress = super::progress::Control::new(enabled);
        progress.report(boundary.progress(), Stage::Prepare, 0, 1)?;
        let overhead = Self::bookkeeping_bytes(self.boundary_capacity);
        self.peak_capacity = overhead;
        PreparedResize::required_bytes(plan.source, plan.output, plan.resize)?;
        let mut call = super::preparation::Call::snapshot(
            &mut self.preparation,
            plan.source_len,
            overhead,
            self.memory_limit,
            &mut self.peak_capacity,
            allocator,
        )?;
        let parent = call.source(plan.source, |bytes, compare| {
            boundary.snapshot_input(bytes, compare)
        })?;
        let key = super::identity::stage(
            Some(parent),
            crate::prod::contract::cache::StageOptions::Resize {
                output: request.output,
            },
        )?;
        if call.take_image(0, key) {
            let image = call.image(0).unwrap();
            let result = boundary.complete(&image.bytes, image.dimensions);
            return call.finish(progress.finish(result, boundary.progress()));
        }
        call.prepare(
            None,
            Some(super::preparation::ResizePreparation {
                source: plan.source,
                output: request.output,
            }),
            [plan.source_len, plan.output_len, 0, 0],
            0,
            &mut self.peak_capacity,
            allocator,
        )?;
        let (_, metadata, scratch) = call.parts();
        let [source, output, _, _] = &mut scratch.buffers;
        let source = ImageView::<Rgba8>::packed(source, plan.source)
            .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
        let output = ImageViewMut::<Rgba8>::packed(output, plan.output)
            .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
        if enabled {
            metadata.expect("requested resize").execute_with_progress(
                source,
                output,
                &mut |completed, total| {
                    progress.report(
                        boundary.progress(),
                        Stage::Resize,
                        u64::from(completed),
                        u64::from(total),
                    )
                },
            )?;
        } else {
            metadata
                .expect("requested resize")
                .execute(source, output)?;
        }
        let content = call.content(0, 1, plan.output);
        call.retain_rgba(0, key, 1, plan.output, content, &mut self.peak_capacity);
        let bytes = call
            .image(0)
            .map_or(call.scratch.buffers[1].as_slice(), |image| &image.bytes);
        let result = boundary.complete(bytes, plan.output);
        call.finish(progress.finish(result, boundary.progress()))
    }
}

impl Plan {
    fn new(request: ResizeRequest, input_len: usize) -> Result<Self, Failure> {
        resize::supported(request.output.resize)?;
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
        Ok(Self {
            source,
            output,
            source_len,
            output_len,
            resize: request.output.resize,
        })
    }
}

pub(super) fn dimensions(
    width: u32,
    height: u32,
    source: bool,
) -> Result<ImageDimensions, Failure> {
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

#[cfg(test)]
mod band_tests;
#[cfg(test)]
mod preparation_tests;
#[cfg(test)]
mod progress_tests;
