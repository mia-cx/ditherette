//! Standalone `apply_effects` and the in-place kernel the processor shares.
//!
//! A leading run of per-channel effects is tabulated per input byte. When that
//! run is the whole chain, no continuous carrier is allocated at all.

use std::collections::TryReserveError;

use crate::{
    image::{contracts::Rgba8Image, ImageBuf, ImageDimensions, ImageView, Rgba8},
    prod::contract::{
        error::{DitheretteError, ErrorCode},
        request::{validate_dimensions, Source, MAX_SOURCE_SIDE},
    },
};

use super::{
    chain::{validate_chain, Effect, EffectContext, Step},
    image::EffectImage,
    recipe::EffectStep,
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
    apply_in_place(
        &mut data,
        source.dimensions(),
        request.effects,
        &request.context,
    )
    .map_err(|_| {
        DitheretteError::new(
            ErrorCode::WasmMemoryUnavailable,
            "wasm",
            "The effect carrier could not be allocated.",
        )
    })?;
    Ok(ImageBuf::from_vec_packed(data, source.dimensions())
        .expect("packed source length matches its dimensions"))
}

/// Bytes `apply_in_place` allocates beyond `data`: zero when every enabled step tabulates.
pub fn carrier_bytes<E: Effect>(steps: &[Step<E>], dimensions: ImageDimensions) -> u64 {
    let tabulates = steps
        .iter()
        .filter(|step| step.enabled)
        .all(|step| step.effect.channel_map().is_some());
    if tabulates {
        0
    } else {
        EffectImage::carrier_bytes(u64::from(dimensions.width()) * u64::from(dimensions.height()))
    }
}

/// Applies an already validated chain to packed RGBA8. Alpha bytes are never written.
pub fn apply_in_place<E: Effect>(
    data: &mut [u8],
    dimensions: ImageDimensions,
    steps: &[Step<E>],
    context: &EffectContext<'_>,
) -> Result<(), TryReserveError> {
    let enabled: Vec<&E> = steps
        .iter()
        .filter(|step| step.enabled)
        .map(|step| &step.effect)
        .collect();
    let tabulated = enabled
        .iter()
        .take_while(|effect| effect.channel_map().is_some())
        .count();
    let tables = ChannelTables::new(
        enabled[..tabulated]
            .iter()
            .filter_map(|effect| effect.channel_map()),
    );
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
    let mut image = EffectImage::try_from_packed(data, dimensions, &tables)?;
    for effect in &enabled[tabulated..] {
        effect.apply(&mut image, context);
    }
    image.write_rgb(data);
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

fn unsupported(path: &str) -> DitheretteError {
    DitheretteError::new(
        ErrorCode::InvalidRequest,
        path,
        "Unsupported request version.",
    )
}
