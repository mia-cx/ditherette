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
    chain::{validate_chain, Effect, EffectContext, PixelMap, Step, MAX_EFFECTS},
    curves::{Spline, MAX_POINTS},
    image::{EffectImage, CARRIER_LIMIT},
    memo::{try_memoized, MEMO_BYTES},
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
        + MAX_POINTS * size_of::<[f32; 2]>()
        + MAX_GROUPS * size_of::<Group>();
    // A boxed map captures at most a spline or a recipe reference, a turn, and a strength.
    let map = size_of::<&EffectStep>() + size_of::<PixelMap<'static>>() + size_of::<Spline>() + 64;
    let palette =
        MAX_PALETTE_ENTRIES * (size_of::<[u8; 3]>() + size_of::<[f32; 3]>() + size_of::<f32>());
    (MAX_EFFECTS * (step + map) + palette + 4 * step) as u64
};

/// Bytes an effects call allocates beyond `data`. A chain that fully tabulates needs only the
/// bookkeeping; otherwise add the carrier, the colour memo, and the largest step scratch.
pub fn carrier_bytes<E: Effect>(steps: &[Step<E>], dimensions: ImageDimensions) -> u64 {
    let tabulates = steps
        .iter()
        .filter(|step| step.enabled)
        .all(|step| step.effect.per_channel());
    if tabulates {
        return BOOKKEEPING_BYTES;
    }
    let scratch = steps
        .iter()
        .filter(|step| step.enabled)
        .map(|step| step.effect.working_bytes())
        .max()
        .unwrap_or(0);
    EffectImage::carrier_bytes(u64::from(dimensions.width()) * u64::from(dimensions.height()))
        + MEMO_BYTES
        + scratch
        + BOOKKEEPING_BYTES
}

/// Replaces each enabled recipe-less `recolour` step with the recipe it would derive from the
/// carrier reaching it, in order. The result is pointwise everywhere, so it memoizes.
pub fn resolve_recolour(
    data: &[u8],
    dimensions: ImageDimensions,
    steps: &[EffectStep],
    context: &EffectContext<'_>,
) -> Result<Vec<EffectStep>, TryReserveError> {
    let mut resolved = Vec::new();
    resolved.try_reserve_exact(steps.len())?;
    resolved.extend_from_slice(steps);
    for index in 0..resolved.len() {
        let BuiltinEffect::Recolour(recolour) = &resolved[index].effect else {
            continue;
        };
        if !resolved[index].enabled || recolour.recipe.is_some() || recolour.strength == 0.0 {
            continue;
        }
        let strength = recolour.strength;
        let image = carrier_after(data, dimensions, &resolved[..index], context)?;
        let recipe = match context.analyses {
            Some(cache) => cache.analyze(&image, context),
            None => analyze(&image, context),
        }?;
        resolved[index].effect = BuiltinEffect::Recolour(Recolour {
            strength,
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
pub fn apply_in_place<E: Effect>(
    data: &mut [u8],
    dimensions: ImageDimensions,
    steps: &[Step<E>],
    context: &EffectContext<'_>,
) -> Result<(), TryReserveError> {
    let (enabled, tabulated) = plan(steps)?;
    let tables = ChannelTables::new(enabled[..tabulated].iter().copied());
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
    carrier(data, dimensions, &enabled, tabulated, &tables, context)?.write_rgb(data);
    Ok(())
}

/// The continuous carrier after an already validated chain: what a step appended to it receives.
pub fn carrier_after<E: Effect>(
    data: &[u8],
    dimensions: ImageDimensions,
    steps: &[Step<E>],
    context: &EffectContext<'_>,
) -> Result<EffectImage, TryReserveError> {
    let (enabled, tabulated) = plan(steps)?;
    let tables = ChannelTables::new(enabled[..tabulated].iter().copied());
    carrier(data, dimensions, &enabled, tabulated, &tables, context)
}

/// Enabled effects in order, and how many lead as a tabulated per-channel run.
fn plan<E: Effect>(steps: &[Step<E>]) -> Result<(Vec<&E>, usize), TryReserveError> {
    let mut enabled = Vec::new();
    enabled.try_reserve_exact(steps.len())?;
    enabled.extend(
        steps
            .iter()
            .filter(|step| step.enabled)
            .map(|step| &step.effect),
    );
    let tabulated = enabled
        .iter()
        .take_while(|effect| effect.per_channel())
        .count();
    Ok((enabled, tabulated))
}

/// Builds the carrier through the tables, then applies the rest, bounding after each step.
/// When every remaining effect is pointwise, each distinct input colour runs the chain once.
fn carrier<E: Effect>(
    data: &[u8],
    dimensions: ImageDimensions,
    enabled: &[&E],
    tabulated: usize,
    tables: &ChannelTables,
    context: &EffectContext<'_>,
) -> Result<EffectImage, TryReserveError> {
    let rest = &enabled[tabulated..];
    if rest.is_empty() {
        return EffectImage::try_from_packed(data, dimensions, tables);
    }
    let mut maps = Vec::new();
    maps.try_reserve_exact(rest.len())?;
    maps.extend(rest.iter().map_while(|effect| effect.pixel_map(context)));
    if maps.len() == rest.len() {
        return try_memoized(data, dimensions, |bytes| {
            let mut rgb = std::array::from_fn(|channel| tables.unit(channel, bytes[channel]));
            for map in &maps {
                rgb = map(rgb).map(|value| value.clamp(-CARRIER_LIMIT, CARRIER_LIMIT));
            }
            rgb
        });
    }
    let mut image = EffectImage::try_from_packed(data, dimensions, tables)?;
    for effect in rest {
        effect.apply(&mut image, context);
        image.bound();
    }
    Ok(image)
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
