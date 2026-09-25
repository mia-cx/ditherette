//! Standalone `apply_effects`. The processor composes effects with `process`.

use crate::{
    image::{contracts::Rgba8Image, ImageView, Rgba8},
    prod::contract::{
        error::{DitheretteError, ErrorCode},
        request::{validate_dimensions, Source, MAX_SOURCE_SIDE},
    },
};

use super::{
    chain::{apply_chain, validate_chain, EffectContext},
    image::EffectImage,
    recipe::EffectStep,
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
    let mut image = EffectImage::from_rgba8(source);
    apply_chain(&mut image, request.effects, &request.context)?;
    Ok(image.to_rgba8())
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
