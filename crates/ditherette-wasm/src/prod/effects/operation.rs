//! Standalone `apply_effects` and the in-place kernel the processor shares.
//!
//! A leading run of per-channel effects is tabulated per input byte. When that
//! run is the whole chain, no continuous carrier is allocated at all. After it,
//! a chain of pointwise effects is memoized by input colour.

use std::{collections::TryReserveError, mem::size_of};

use crate::{
    image::{contracts::Rgba8Image, ImageBuf, ImageDimensions, ImageView, Rgba8},
    prod::contract::{
        error::{DitheretteError, ErrorCode},
        request::{validate_dimensions, Source, MAX_PALETTE_ENTRIES, MAX_SOURCE_SIDE},
    },
};

use super::{
    chain::{
        validate_chain, Effect, EffectContext, Needs, Planned, PreparedPointwise,
        PreparedPointwiseState, Step, MAX_EFFECTS, PREPARED_STEP_BYTES,
    },
    curves::MAX_POINTS,
    image::{byte, EffectImage},
    mask::{MaskCurve, PreparedMask},
    memo::{byte_memo_bytes, try_memoized, try_memoized_bytes, FLOAT_MEMO_BYTES},
    recipe::{BuiltinEffect, EffectStep},
    recolour::{Group, Recolour, RecolourRecipe, MAX_GROUPS},
    recolour_analysis::analyze,
    table::ChannelTables,
};

/// Version of the standalone `apply_effects` request shape.
pub const EFFECTS_VERSION: u32 = 1;

/// Standalone chain request. The source is borrowed and never modified.
#[derive(Debug, Clone, Copy)]
pub struct EffectsRequest<'a> {
    pub version: u32,
    pub source: Source<'a>,
    pub effects: &'a [EffectStep],
    pub context: EffectContext<'a>,
}

/// Applies the ordered chain and returns owned full-colour RGBA8 at the source size.
pub fn apply_effects(request: EffectsRequest<'_>) -> Result<Rgba8Image, DitheretteError> {
    if request.version != EFFECTS_VERSION {
        return Err(unsupported("version"));
    }
    validate_chain(request.effects, &request.context)?;
    let source = source_view(request.source)?;
    let mut data = source.data().to_vec();
    let steps = resolve_recolour(
        &data,
        source.dimensions(),
        request.effects,
        &request.context,
    )
    .map_err(|_| unavailable())?;
    apply_in_place(&mut data, source.dimensions(), &steps, &request.context)
        .map_err(|_| unavailable())?;
    Ok(ImageBuf::from_vec_packed(data, source.dimensions())
        .expect("packed source length matches its dimensions"))
}

/// Upper bound on the small allocations an effects call makes besides the carrier and memo:
/// the resolved step copy, the enabled-step list, prepared pixel maps, and analysis's palette
/// tables, sectors, groups, and recipes. Every count is capped by validation.
pub const BOOKKEEPING_BYTES: u64 = {
    let step = size_of::<EffectStep>()
        + 3 * MAX_POINTS * size_of::<[f32; 2]>()
        + MAX_GROUPS * size_of::<Group>();
    let palette =
        MAX_PALETTE_ENTRIES * (size_of::<[u8; 3]>() + size_of::<[f32; 3]>() + size_of::<f32>());
    (MAX_EFFECTS * (step + size_of::<Planned<'static, BuiltinEffect>>() + PREPARED_STEP_BYTES)
        + palette
        + 4 * step) as u64
};

/// Per-pixel bytes a masked step that is not pointwise needs on the carrier path: a copy of its
/// input and one strength.
const MASKED_CARRIER_BYTES_PER_PIXEL: u64 = (size_of::<[f32; 3]>() + size_of::<f32>()) as u64;

/// True when a step joins the leading per-channel tables. A mask reads all three channels.
fn tabulates<E: Effect>(step: &Planned<'_, E>) -> bool {
    step.mask.is_empty() && step.effect.per_channel()
}

/// The input copy and strengths the carrier path allocates for masked steps that are not pointwise.
fn masked_carrier_bytes<'a, E: Effect + 'a>(
    mut enabled: impl Iterator<Item = Planned<'a, E>>,
    pixels: u64,
) -> u64 {
    if enabled.any(|step| !step.mask.is_empty() && !step.effect.pointwise()) {
        pixels * MASKED_CARRIER_BYTES_PER_PIXEL
    } else {
        0
    }
}

/// Enabled steps without allocating, for memory estimates.
fn enabled_steps<E, M: AsRef<[MaskCurve]>>(
    steps: &[Step<E, M>],
) -> impl Iterator<Item = Planned<'_, E>> + Clone {
    steps
        .iter()
        .filter(|step| step.enabled)
        .map(|step| step.planned())
}

/// Bytes terminal application allocates beyond `data`. Pointwise tails use the adaptive byte
/// memo; only chains that need a continuous image pay for a full carrier.
pub fn carrier_bytes<E: Effect, M: AsRef<[MaskCurve]>>(
    steps: &[Step<E, M>],
    dimensions: ImageDimensions,
) -> u64 {
    let enabled = enabled_steps(steps);
    let enabled_count = enabled.clone().count();
    let tabulated = enabled.clone().take_while(tabulates).count();
    if tabulated == enabled_count {
        return BOOKKEEPING_BYTES;
    }
    let scratch = enabled
        .clone()
        .map(|step| step.effect.working_bytes())
        .max()
        .unwrap_or(0);
    let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
    if enabled
        .clone()
        .skip(tabulated)
        .all(|step| step.effect.pointwise())
    {
        return byte_memo_bytes(pixels) + scratch + BOOKKEEPING_BYTES;
    }
    EffectImage::carrier_bytes(pixels)
        + if scratch > 0 { FLOAT_MEMO_BYTES } else { 0 }
        + masked_carrier_bytes(enabled, pixels)
        + scratch
        + BOOKKEEPING_BYTES
}

/// Bytes `carrier_after` allocates beyond `data`: the continuous image, optional float memo,
/// masked-step scratch, and the largest effect scratch allocation.
pub fn carrier_after_bytes<E: Effect, M: AsRef<[MaskCurve]>>(
    steps: &[Step<E, M>],
    dimensions: ImageDimensions,
) -> u64 {
    let enabled = enabled_steps(steps);
    let tabulated = enabled.clone().take_while(tabulates).count();
    // Resolving a recipe-less recolour builds the carrier for the pointwise steps before it, which
    // memoizes even when the whole chain is not pointwise, so any step past the tables may need it.
    let memo = if enabled.clone().count() > tabulated {
        FLOAT_MEMO_BYTES
    } else {
        0
    };
    let pixels = u64::from(dimensions.width()) * u64::from(dimensions.height());
    let masked = masked_carrier_bytes(enabled.clone(), pixels);
    let scratch = enabled
        .map(|step| step.effect.working_bytes())
        .max()
        .unwrap_or(0);
    EffectImage::carrier_bytes(pixels) + memo + masked + scratch + BOOKKEEPING_BYTES
}

/// A step as it runs: the caller's own effect, or a recolour step with the recipe it derived.
/// Borrowing the rest means resolution copies no curve points or recipes.
pub enum Resolved<'a> {
    Given(&'a BuiltinEffect),
    Recolour(Recolour),
}

impl Effect for Resolved<'_> {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        match self {
            Self::Given(effect) => effect.validate(path),
            Self::Recolour(effect) => effect.validate(path),
        }
    }

    fn needs(&self) -> Needs {
        match self {
            Self::Given(effect) => effect.needs(),
            Self::Recolour(effect) => effect.needs(),
        }
    }

    fn check_context(
        &self,
        context: &EffectContext<'_>,
        path: &str,
    ) -> Result<(), DitheretteError> {
        match self {
            Self::Given(effect) => effect.check_context(context, path),
            Self::Recolour(effect) => effect.check_context(context, path),
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        match self {
            Self::Given(effect) => effect.apply(image, context),
            Self::Recolour(effect) => effect.apply(image, context),
        }
    }

    fn apply_masked(
        &self,
        image: &mut EffectImage,
        input: &[[f32; 3]],
        strengths: &[f32],
        context: &EffectContext<'_>,
    ) {
        match self {
            Self::Given(effect) => effect.apply_masked(image, input, strengths, context),
            Self::Recolour(effect) => effect.apply_masked(image, input, strengths, context),
        }
    }

    fn map_prepared_masked(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        strength: f32,
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        match self {
            Self::Given(effect) => effect.map_prepared_masked(prepared, rgb, strength, context),
            Self::Recolour(effect) => effect.map_prepared_masked(prepared, rgb, strength, context),
        }
    }

    fn per_channel(&self) -> bool {
        match self {
            Self::Given(effect) => effect.per_channel(),
            Self::Recolour(effect) => effect.per_channel(),
        }
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        match self {
            Self::Given(effect) => effect.map_channel(channel, value),
            Self::Recolour(effect) => effect.map_channel(channel, value),
        }
    }

    fn map_prepared_channel(
        &self,
        prepared: &PreparedPointwise,
        channel: usize,
        value: f32,
    ) -> f32 {
        match self {
            Self::Given(effect) => effect.map_prepared_channel(prepared, channel, value),
            Self::Recolour(effect) => effect.map_prepared_channel(prepared, channel, value),
        }
    }

    fn working_bytes(&self) -> u64 {
        match self {
            Self::Given(effect) => effect.working_bytes(),
            Self::Recolour(effect) => effect.working_bytes(),
        }
    }

    fn pointwise(&self) -> bool {
        match self {
            Self::Given(effect) => effect.pointwise(),
            Self::Recolour(effect) => effect.pointwise(),
        }
    }

    fn map_pixel(&self, rgb: [f32; 3], context: &EffectContext<'_>) -> [f32; 3] {
        match self {
            Self::Given(effect) => effect.map_pixel(rgb, context),
            Self::Recolour(effect) => effect.map_pixel(rgb, context),
        }
    }

    fn prepare_pointwise(&self) -> PreparedPointwise {
        match self {
            Self::Given(effect) => effect.prepare_pointwise(),
            Self::Recolour(effect) => effect.prepare_pointwise(),
        }
    }

    fn map_prepared(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        match self {
            Self::Given(effect) => effect.map_prepared(prepared, rgb, context),
            Self::Recolour(effect) => effect.map_prepared(prepared, rgb, context),
        }
    }

    fn map_prepared_linear(
        &self,
        prepared: &PreparedPointwise,
        linear: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        match self {
            Self::Given(effect) => effect.map_prepared_linear(prepared, linear, context),
            Self::Recolour(effect) => effect.map_prepared_linear(prepared, linear, context),
        }
    }
}

/// A step as it runs, borrowing the caller's mask.
pub type ResolvedStep<'a> = Step<Resolved<'a>, &'a [MaskCurve]>;

/// Replaces each enabled recipe-less `recolour` step with the recipe it would derive from the
/// carrier reaching it, in order. The result is pointwise everywhere, so it memoizes.
/// Analysis reads the unmasked carrier; the step's mask still applies to its result.
pub fn resolve_recolour<'a>(
    data: &[u8],
    dimensions: ImageDimensions,
    steps: &'a [EffectStep],
    context: &EffectContext<'_>,
) -> Result<Vec<ResolvedStep<'a>>, TryReserveError> {
    let mut resolved = Vec::new();
    resolved.try_reserve_exact(steps.len())?;
    resolved.extend(steps.iter().map(|step| Step {
        enabled: step.enabled,
        mask: step.mask.as_slice(),
        effect: Resolved::Given(&step.effect),
    }));
    for index in 0..resolved.len() {
        let Resolved::Given(BuiltinEffect::Recolour(recolour)) = resolved[index].effect else {
            continue;
        };
        if !resolved[index].enabled || recolour.recipe.is_some() || recolour.strength == 0.0 {
            continue;
        }
        let image = carrier_after(data, dimensions, &resolved[..index], context)?;
        let recipe = match context.analyses {
            Some(cache) => cache.analyze(&image, context),
            None => analyze(&image, context),
        }?;
        resolved[index].effect = Resolved::Recolour(Recolour {
            strength: recolour.strength,
            recipe: Some(recipe),
        });
    }
    Ok(resolved)
}

/// Analysis request: the image a recolour step would receive is `source` after `effects`.
#[derive(Debug, Clone, Copy)]
pub struct AnalyzeRequest<'a> {
    pub version: u32,
    pub source: Source<'a>,
    pub effects: &'a [EffectStep],
    pub context: EffectContext<'a>,
}

/// Runs `effects` on the source, then analyses the result against the context palette and space.
/// The recipe equals what a recipe-less `recolour` step appended to `effects` would derive.
pub fn analyze_recolour(request: AnalyzeRequest<'_>) -> Result<RecolourRecipe, DitheretteError> {
    if request.version != EFFECTS_VERSION {
        return Err(unsupported("version"));
    }
    validate_chain(request.effects, &request.context)?;
    if request.context.colors().next().is_none() {
        return Err(DitheretteError::new(
            ErrorCode::InvalidRequest,
            "context.palette",
            "Analysis requires a visible palette colour.",
        ));
    }
    if request.context.space.is_none() {
        return Err(DitheretteError::new(
            ErrorCode::InvalidRequest,
            "context.space",
            "Analysis requires a working space.",
        ));
    }
    let source = source_view(request.source)?;
    let steps = resolve_recolour(
        source.data(),
        source.dimensions(),
        request.effects,
        &request.context,
    )
    .map_err(|_| unavailable())?;
    let image = carrier_after(source.data(), source.dimensions(), &steps, &request.context)
        .map_err(|_| unavailable())?;
    match request.context.analyses {
        Some(cache) => cache.analyze(&image, &request.context),
        None => analyze(&image, &request.context),
    }
    .map_err(|_| unavailable())
}

/// Applies an already validated chain to packed RGBA8. Alpha bytes are never written.
pub fn apply_in_place<E: Effect, M: AsRef<[MaskCurve]>>(
    data: &mut [u8],
    dimensions: ImageDimensions,
    steps: &[Step<E, M>],
    context: &EffectContext<'_>,
) -> Result<(), TryReserveError> {
    let (enabled, tabulated) = plan(steps)?;
    let tables = ChannelTables::new(enabled[..tabulated].iter().map(|step| step.effect));
    if tabulated == enabled.len() {
        if tabulated > 0 {
            let [red, green, blue] = tables.bytes();
            for pixel in data.chunks_exact_mut(4) {
                pixel[0] = red[pixel[0] as usize];
                pixel[1] = green[pixel[1] as usize];
                pixel[2] = blue[pixel[2] as usize];
            }
        }
        return Ok(());
    }
    let rest = &enabled[tabulated..];
    if rest.iter().all(|step| step.effect.pointwise()) {
        let prepared = PreparedPointwiseState::try_new(rest, &tables)?;
        try_memoized_bytes(data, |bytes| prepared.map(bytes, context).map(byte))?;
        return Ok(());
    }
    carrier(data, dimensions, &enabled, tabulated, &tables, context)?.write_rgb(data);
    Ok(())
}

/// The continuous carrier after an already validated chain: what a step appended to it receives.
pub fn carrier_after<E: Effect, M: AsRef<[MaskCurve]>>(
    data: &[u8],
    dimensions: ImageDimensions,
    steps: &[Step<E, M>],
    context: &EffectContext<'_>,
) -> Result<EffectImage, TryReserveError> {
    let (enabled, tabulated) = plan(steps)?;
    let tables = ChannelTables::new(enabled[..tabulated].iter().map(|step| step.effect));
    carrier(data, dimensions, &enabled, tabulated, &tables, context)
}

/// Writes each pixel's strength for a validated `mask`, read from the carrier after an already
/// validated chain, as grey `round(strength * 255)` in `output`. Alpha comes from `data`.
pub fn write_mask<E: Effect, M: AsRef<[MaskCurve]>>(
    data: &[u8],
    dimensions: ImageDimensions,
    steps: &[Step<E, M>],
    mask: &[MaskCurve],
    context: &EffectContext<'_>,
    output: &mut [u8],
) -> Result<(), TryReserveError> {
    let prepared = PreparedMask::new(mask);
    let image = carrier_after(data, dimensions, steps, context)?;
    for ((pixel, rgb), alpha) in output.chunks_exact_mut(4).zip(&image.rgb).zip(&image.alpha) {
        let grey = byte(prepared.strength(mask, *rgb));
        pixel.copy_from_slice(&[grey, grey, grey, *alpha]);
    }
    Ok(())
}

/// Enabled steps in order, and how many lead as a tabulated per-channel run.
fn plan<E: Effect, M: AsRef<[MaskCurve]>>(
    steps: &[Step<E, M>],
) -> Result<(Vec<Planned<'_, E>>, usize), TryReserveError> {
    let mut enabled = Vec::new();
    enabled.try_reserve_exact(steps.len())?;
    enabled.extend(enabled_steps(steps));
    let tabulated = enabled.iter().take_while(|step| tabulates(step)).count();
    Ok((enabled, tabulated))
}

/// Builds the carrier through the tables, then applies the rest, bounding after each step.
/// When every remaining effect is pointwise, each distinct input colour runs the chain once.
fn carrier<E: Effect>(
    data: &[u8],
    dimensions: ImageDimensions,
    enabled: &[Planned<'_, E>],
    tabulated: usize,
    tables: &ChannelTables,
    context: &EffectContext<'_>,
) -> Result<EffectImage, TryReserveError> {
    let rest = &enabled[tabulated..];
    if rest.is_empty() {
        return EffectImage::try_from_packed(data, dimensions, tables);
    }
    if rest.iter().all(|step| step.effect.pointwise()) {
        let prepared = PreparedPointwiseState::try_new(rest, tables)?;
        return try_memoized(data, dimensions, |bytes| prepared.map(bytes, context));
    }
    let mut image = EffectImage::try_from_packed(data, dimensions, tables)?;
    for step in rest {
        apply_step(&mut image, step, context)?;
        image.bound();
    }
    Ok(image)
}

/// One step on the continuous carrier. A masked pointwise step maps pixel by pixel; any other
/// masked step needs a copy of its input and its strengths, reserved fallibly.
fn apply_step<E: Effect>(
    image: &mut EffectImage,
    step: &Planned<'_, E>,
    context: &EffectContext<'_>,
) -> Result<(), TryReserveError> {
    if step.mask.is_empty() {
        step.effect.apply(image, context);
        return Ok(());
    }
    let mask = PreparedMask::new(step.mask);
    if step.effect.pointwise() {
        let map = step.effect.prepare_pointwise();
        for rgb in &mut image.rgb {
            let strength = mask.strength(step.mask, *rgb);
            *rgb = step
                .effect
                .map_prepared_masked(&map, *rgb, strength, context);
        }
        return Ok(());
    }
    let mut input = Vec::new();
    input.try_reserve_exact(image.rgb.len())?;
    input.extend_from_slice(&image.rgb);
    let mut strengths = Vec::new();
    strengths.try_reserve_exact(input.len())?;
    strengths.extend(input.iter().map(|&rgb| mask.strength(step.mask, rgb)));
    step.effect.apply_masked(image, &input, &strengths, context);
    Ok(())
}

fn source_view(source: Source<'_>) -> Result<ImageView<'_, Rgba8>, DitheretteError> {
    let dimensions = validate_dimensions(
        source.width,
        source.height,
        MAX_SOURCE_SIDE,
        "source",
        ErrorCode::InvalidImage,
    )?;
    ImageView::packed(source.data, dimensions).map_err(|_| {
        DitheretteError::new(
            ErrorCode::InvalidImage,
            "source.data",
            "RGBA8 byte length does not match dimensions.",
        )
    })
}

fn unavailable() -> DitheretteError {
    DitheretteError::new(
        ErrorCode::WasmMemoryUnavailable,
        "wasm",
        "The effect carrier could not be allocated.",
    )
}

fn unsupported(path: &str) -> DitheretteError {
    DitheretteError::new(
        ErrorCode::InvalidRequest,
        path,
        "Unsupported request version.",
    )
}
