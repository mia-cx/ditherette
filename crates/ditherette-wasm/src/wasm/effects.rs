//! Ordered effect chains through the borrowed input and caught result sinks.
//!
//! The package validates effects with precise paths, then passes them as JSON.
//! Rust decodes and validates again; a rejection here reports the `effects` path.

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use super::{
    fields::parse_space,
    processor::{dimension, restore_ready, status, take_ready, JsBoundary},
    quantize::{read_palette, PALETTE_SLOTS},
};
use crate::{
    image::contracts::PaletteEntry,
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::MAX_SOURCE_SIDE,
        },
        effects::{decode_effects, EffectContext, EffectStep},
        pipeline::effects::EffectsRequest,
    },
};

/// Context space tag for "no working space supplied".
const NO_SPACE: f64 = -1.0;

/// `palette` is undefined when the caller supplied no palette context.
/// `space` is a working-space tag or -1. Success writes durable RGBA8 to the sink.
#[wasm_bindgen(js_name = privateApplyEffects)]
pub fn private_apply_effects(
    input: &Uint8Array,
    source_width: f64,
    source_height: f64,
    effects: &str,
    palette: &JsValue,
    space: f64,
    result_sink: &JsValue,
) -> u32 {
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        let source_width = dimension(
            source_width,
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceWidth,
        )?;
        let source_height = dimension(
            source_height,
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceHeight,
        )?;
        let effects = decode(effects)?;
        let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
        let count = if palette.is_undefined() {
            0
        } else {
            read_palette(palette, &mut entries)?
        };
        let space = if space == NO_SPACE {
            None
        } else {
            Some(parse_space(space).ok_or(Failure::new(
                ErrorCode::InvalidSettings,
                ErrorPath::ContextSpace,
            ))?)
        };
        processor.apply_effects(
            EffectsRequest {
                source_width,
                source_height,
                effects: &effects,
                context: EffectContext {
                    palette: &entries[..count],
                    space,
                },
            },
            &mut JsBoundary::new(input, result_sink)?,
        )
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}

/// Decodes the wrapper's normalized effects JSON.
pub(super) fn decode(effects: &str) -> Result<Vec<EffectStep>, Failure> {
    decode_effects(effects)
        .map_err(|_| Failure::new(ErrorCode::InvalidSettings, ErrorPath::Effects))
}
