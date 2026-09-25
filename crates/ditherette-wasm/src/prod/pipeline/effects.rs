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
    prod::contract::cache::Identity,
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            lifecycle::Stage,
        },
        effects::{
            apply_in_place, carrier_after, carrier_bytes, chain::validate_chain,
            recolour::RecolourRecipe, recolour_analysis, resolve_recolour, EffectContext,
            EffectImage, EffectStep,
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

/// Extra bytes a recipe-v2 `process` owns: the retained raw snapshot and the chain's scratch.
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
    let steps = resolve_recolour(output, dimensions, request.effects, &request.context)
        .map_err(|_| unavailable())?;
    apply_in_place(output, dimensions, &steps, &request.context).map_err(|_| unavailable())?;
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
    // The carrier always exists here; earlier steps may add scratch; analysis adds its samples.
    let carrier = carrier_bytes(request.effects, dimensions)
        .max(EffectImage::carrier_bytes(pixels))
        + recolour_analysis::ANALYSIS_BYTES;
    call.charge_working_capacity(carrier, peak)?;
    call.prepare(None, None, [len, 0, 0, 0], 0, peak, allocator)?;
    progress.report(boundary.progress(), Stage::Effects, 0, pixels)?;
    let source = &call.scratch.buffers[0];
    let steps = resolve_recolour(source, dimensions, request.effects, &request.context)
        .map_err(|_| unavailable())?;
    let image =
        carrier_after(source, dimensions, &steps, &request.context).map_err(|_| unavailable())?;
    let recipe = match request.context.analyses {
        Some(cache) => cache.analyze(&image, &request.context),
        None => recolour_analysis::analyze(&image, &request.context),
    }
    .map_err(|_| unavailable())?;
    drop(image);
    call.release_working_capacity(carrier);
    progress.report(boundary.progress(), Stage::Effects, pixels, pixels)?;
    call.finish(progress.finish(Ok(recipe), boundary.progress()))
}

/// The raw source and chain key of the last successful recipe-v2 call. While both repeat,
/// that call's effected snapshot is still the source `process` compares against.
#[derive(Debug)]
pub(super) struct EffectsSource {
    raw: Vec<u8>,
    dimensions: ImageDimensions,
    key: Identity,
}

impl EffectsSource {
    /// Hashes the enabled steps and any context they read, independent of the source.
    pub(super) fn key(
        effects: &[EffectStep],
        context: &EffectContext<'_>,
    ) -> Result<Identity, Failure> {
        super::identity::effects(Identity([0; 32]), effects, context)
    }
}

/// `process` input whose bytes are the caller's source after the effect chain.
/// It keeps its own raw snapshot, so an unchanged source and chain skip the effects entirely
/// and hand `process` the identical bytes it snapshotted last time.
pub(super) struct EffectedInput<'a, B> {
    inner: &'a mut B,
    effects: &'a [EffectStep],
    context: EffectContext<'a>,
    dimensions: ImageDimensions,
    progress: Control,
    raw: Vec<u8>,
    key: Identity,
    previous: Option<Identity>,
}

impl<'a, B: InputBoundary> EffectedInput<'a, B> {
    pub(super) fn new(
        inner: &'a mut B,
        effects: &'a [EffectStep],
        context: EffectContext<'a>,
        dimensions: ImageDimensions,
        key: Identity,
        previous: Option<EffectsSource>,
    ) -> Self {
        let progress = Control::new(inner.progress().is_some());
        let (raw, previous) = match previous {
            Some(source) if source.dimensions == dimensions => (source.raw, Some(source.key)),
            _ => (Vec::new(), None),
        };
        Self {
            inner,
            effects,
            context,
            dimensions,
            progress,
            raw,
            key,
            previous,
        }
    }

    /// Keeps the raw snapshot for the next call. The caller retains it only after success.
    pub(super) fn into_source(self) -> EffectsSource {
        EffectsSource {
            raw: self.raw,
            dimensions: self.dimensions,
            key: self.key,
        }
    }

    /// Refreshes the raw snapshot; true when the current input equals the retained one.
    fn snapshot_raw(&mut self) -> Result<bool, Failure> {
        let len = self.inner.input_len()?;
        if self.raw.len() == len {
            return self.inner.snapshot_input(&mut self.raw, true);
        }
        self.previous = None;
        self.raw.clear();
        self.raw
            .try_reserve_exact(len)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm))?;
        self.raw.resize(len, 0);
        self.inner.snapshot_input(&mut self.raw, false)?;
        Ok(false)
    }

    fn effected_copy(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        let pixels = u64::from(self.dimensions.width()) * u64::from(self.dimensions.height());
        self.progress
            .report(self.inner.progress(), Stage::Effects, 0, pixels)?;
        destination.copy_from_slice(&self.raw);
        let steps = resolve_recolour(destination, self.dimensions, self.effects, &self.context)
            .map_err(|_| unavailable())?;
        apply_in_place(destination, self.dimensions, &steps, &self.context)
            .map_err(|_| unavailable())?;
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
        self.snapshot_raw()?;
        self.effected_copy(destination)
    }

    /// `compare` means `destination` still holds the last snapshot. When the raw source and
    /// chain repeat, that snapshot is the effected source of the previous successful call.
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        let same_raw = self.snapshot_raw()?;
        if compare && same_raw && self.previous == Some(self.key) {
            return Ok(true);
        }
        self.effected_copy(destination)?;
        Ok(false)
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

fn unavailable() -> Failure {
    Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}
