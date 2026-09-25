//! Bounded ownership around the ordered effects kernel.
//!
//! `run` serves `applyEffects` with the shared source snapshot and image LRU.
//! `EffectedInput` applies the same chain while `process` snapshots its source,
//! so unchanged effects keep the downstream resize and indexed caches warm.

use std::mem::size_of;

use super::{
    preparation::{Call, Store},
    processor::{dimensions, Allocator, Boundary, InputBoundary},
    progress::{Callback, Control},
    quantize::{IndexedMetadataRef, QuantizeBoundary},
};
use crate::{
    image::{ImageDimensions, ImageFormat, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::Stage,
        },
        effects::{
            apply_in_place, carrier_after, carrier_bytes, chain::validate_chain,
            recolour::RecolourRecipe, recolour_analysis, EffectContext, EffectImage, EffectStep,
        },
    },
};

/// Typed native call. The adapter decodes `effects` before constructing it.
#[derive(Clone, Copy)]
pub struct EffectsRequest<'a> {
    pub source_width: u32,
    pub source_height: u32,
    pub effects: &'a [EffectStep],
    pub context: EffectContext<'a>,
}

/// Maps reference validation to static private paths. Adapters report finer paths first.
pub fn validate(effects: &[EffectStep], context: &EffectContext<'_>) -> Result<(), Failure> {
    validate_chain(effects, context).map_err(|error| {
        let path = match error.path.as_str() {
            "context.palette" => ErrorPath::ContextPalette,
            "context.space" => ErrorPath::ContextSpace,
            _ => ErrorPath::Effects,
        };
        Failure::new(error.code, path)
    })
}

/// Extra call-owned bytes `process` needs to apply effects while snapshotting its source.
pub(super) fn process_capacity_bytes(effects: &[EffectStep], dimensions: ImageDimensions) -> u64 {
    let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
    pixels * Rgba8::CHANNEL_COUNT as u64 + carrier_bytes(effects, dimensions)
}

pub(super) fn run<B: Boundary, A: Allocator>(
    request: EffectsRequest<'_>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut Store,
) -> Result<B::Output, Failure> {
    validate(request.effects, &request.context)?;
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    let mut progress = Control::new(boundary.progress().is_some());
    progress.report(boundary.progress(), Stage::Prepare, 0, 1)?;
    let owned = overhead
        .checked_add(size_of::<EffectsRequest<'_>>() as u64)
        .ok_or_else(memory_limit)?;
    let mut call = Call::snapshot(store, len, owned, limit, peak, allocator)?;
    let parent = call.source(dimensions, |bytes, compare| {
        boundary.snapshot_input(bytes, compare)
    })?;
    let key = super::identity::effects(parent, request.effects, &request.context)?;
    if call.take_image(1, key) {
        let image = call.image(1).unwrap();
        let result = boundary.complete(&image.bytes, image.dimensions);
        return call.finish(progress.finish(result, boundary.progress()));
    }
    let carrier = carrier_bytes(request.effects, dimensions);
    call.charge_working_capacity(carrier, peak)?;
    call.prepare(None, None, [len, len, 0, 0], 0, peak, allocator)?;
    let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
    progress.report(boundary.progress(), Stage::Effects, 0, pixels)?;
    let [source, output, _, _] = &mut call.scratch.buffers;
    output.copy_from_slice(source);
    apply_in_place(output, dimensions, request.effects, &request.context)
        .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))?;
    progress.report(boundary.progress(), Stage::Effects, pixels, pixels)?;
    call.release_working_capacity(carrier);
    call.retain_rgba(1, key, 1, dimensions, peak);
    let bytes = call
        .image(1)
        .map_or(call.scratch.buffers[1].as_slice(), |image| &image.bytes);
    let result = boundary.complete(bytes, dimensions);
    call.finish(progress.finish(result, boundary.progress()))
}

/// Analysis of the image a recolour step would receive. The source snapshot follows `run`;
/// the continuous carrier is charged while it lives. Only the recipe leaves the call.
pub(super) fn analyze<B: InputBoundary, A: Allocator>(
    request: EffectsRequest<'_>,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut Store,
) -> Result<RecolourRecipe, Failure> {
    validate(request.effects, &request.context)?;
    if request.context.colors().next().is_none() {
        return Err(Failure::new(
            ErrorCode::InvalidRequest,
            ErrorPath::ContextPalette,
        ));
    }
    if request.context.space.is_none() {
        return Err(Failure::new(
            ErrorCode::InvalidRequest,
            ErrorPath::ContextSpace,
        ));
    }
    let dimensions = dimensions(request.source_width, request.source_height, true)?;
    let len = dimensions
        .storage_len::<Rgba8>()
        .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::Source))?;
    if boundary.input_len()? != len {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData));
    }
    let mut progress = Control::new(boundary.progress().is_some());
    progress.report(boundary.progress(), Stage::Prepare, 0, 1)?;
    let owned = overhead
        .checked_add(size_of::<EffectsRequest<'_>>() as u64)
        .ok_or_else(memory_limit)?;
    let mut call = Call::snapshot(store, len, owned, limit, peak, allocator)?;
    call.source(dimensions, |bytes, compare| {
        boundary.snapshot_input(bytes, compare)
    })?;
    let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
    let carrier = EffectImage::carrier_bytes(pixels);
    call.charge_working_capacity(carrier, peak)?;
    call.prepare(None, None, [len, 0, 0, 0], 0, peak, allocator)?;
    progress.report(boundary.progress(), Stage::Effects, 0, pixels)?;
    let image = carrier_after(
        &call.scratch.buffers[0],
        dimensions,
        request.effects,
        &request.context,
    )
    .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))?;
    let recipe = match request.context.analyses {
        Some(cache) => cache.analyze(&image, &request.context),
        None => recolour_analysis::analyze(&image, &request.context),
    };
    drop(image);
    call.release_working_capacity(carrier);
    progress.report(boundary.progress(), Stage::Effects, pixels, pixels)?;
    call.finish(progress.finish(Ok(recipe), boundary.progress()))
}

/// `process` input whose bytes are the caller's source after the effect chain.
/// Its snapshot compares effected bytes, so an unchanged chain reuses the source identity.
pub(super) struct EffectedInput<'a, B> {
    inner: &'a mut B,
    effects: &'a [EffectStep],
    context: EffectContext<'a>,
    dimensions: ImageDimensions,
    progress: Control,
}

impl<'a, B: InputBoundary> EffectedInput<'a, B> {
    pub(super) fn new(
        inner: &'a mut B,
        effects: &'a [EffectStep],
        context: EffectContext<'a>,
        dimensions: ImageDimensions,
    ) -> Self {
        let progress = Control::new(inner.progress().is_some());
        Self {
            inner,
            effects,
            context,
            dimensions,
            progress,
        }
    }

    fn effected_copy(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        let pixels = u64::from(self.dimensions.width()) * u64::from(self.dimensions.height());
        self.progress
            .report(self.inner.progress(), Stage::Effects, 0, pixels)?;
        self.inner.copy_input(destination)?;
        apply_in_place(destination, self.dimensions, self.effects, &self.context)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))?;
        self.progress
            .report(self.inner.progress(), Stage::Effects, pixels, pixels)
    }
}

impl<B: InputBoundary> InputBoundary for EffectedInput<'_, B> {
    fn progress(&mut self) -> Option<&mut dyn Callback> {
        self.inner.progress()
    }

    fn input_len(&mut self) -> Result<usize, Failure> {
        self.inner.input_len()
    }

    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        self.effected_copy(destination)
    }

    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        if !compare {
            self.effected_copy(destination)?;
            return Ok(false);
        }
        let mut scratch = Vec::new();
        scratch
            .try_reserve_exact(destination.len())
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))?;
        scratch.resize(destination.len(), 0);
        let result = self.effected_copy(&mut scratch);
        let equal = result.is_ok() && scratch == destination;
        if result.is_ok() && !equal {
            destination.copy_from_slice(&scratch);
        }
        // Release the comparison copy before `process` reserves its own working buffers.
        drop(scratch);
        result.map(|()| equal)
    }
}

impl<B: QuantizeBoundary> QuantizeBoundary for EffectedInput<'_, B> {
    type Output = B::Output;

    fn capacity_bytes(&self) -> u64 {
        self.inner.capacity_bytes()
    }

    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        palette: IndexedMetadataRef<'_>,
    ) -> Result<Self::Output, Failure> {
        self.inner.complete(indices, dimensions, palette)
    }
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
