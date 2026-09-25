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

#[wasm_bindgen(module = "/src/wasm/effects_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = completeRecipe)]
    fn complete_recipe(text: &str, sink: &JsValue) -> Result<(), JsValue>;
}

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
    let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
    let result = (|| {
        let (request, steps) = parse(
            source_width,
            source_height,
            effects,
            palette,
            space,
            &mut entries,
        )?;
        processor.apply_effects(
            EffectsRequest {
                effects: &steps,
                ..request
            },
            &mut JsBoundary::new(input, result_sink)?,
        )
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}

/// Analyses `input` after `effects` for a recolour step. Success writes the recipe object to the sink.
/// Takes the same arguments as `privateApplyEffects`; the palette and space are required.
#[wasm_bindgen(js_name = privateAnalyzeRecolour)]
pub fn private_analyze_recolour(
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
    let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
    let result = (|| {
        let (request, steps) = parse(
            source_width,
            source_height,
            effects,
            palette,
            space,
            &mut entries,
        )?;
        let recipe = processor.analyze_recolour(
            EffectsRequest {
                effects: &steps,
                ..request
            },
            &mut JsBoundary::new(input, result_sink)?,
        )?;
        let text = serde_json::to_string(&recipe)
            .map_err(|_| Failure::new(ErrorCode::Runtime, ErrorPath::Control))?;
        complete_recipe(&text, result_sink)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Output))
    })();
    restore_ready(processor);
    result.map_or_else(status, |()| 0)
}

/// Shared argument decoding. The returned request borrows `entries`; its effects are separate.
fn parse<'a>(
    source_width: f64,
    source_height: f64,
    effects: &str,
    palette: &JsValue,
    space: f64,
    entries: &'a mut [PaletteEntry; PALETTE_SLOTS],
) -> Result<(EffectsRequest<'a>, Vec<EffectStep>), Failure> {
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
    let steps = decode(effects)?;
    let count = if palette.is_undefined() {
        0
    } else {
        read_palette(palette, entries)?
    };
    let space = if space == NO_SPACE {
        None
    } else {
        Some(parse_space(space).ok_or(Failure::new(
            ErrorCode::InvalidSettings,
            ErrorPath::ContextSpace,
        ))?)
    };
    Ok((
        EffectsRequest {
            source_width,
            source_height,
            effects: &[],
            context: EffectContext {
                palette: &entries[..count],
                space,
                analyses: None,
            },
        },
        steps,
    ))
}

/// Decodes the wrapper's normalized effects JSON.
pub(super) fn decode(effects: &str) -> Result<Vec<EffectStep>, Failure> {
    decode_effects(effects)
        .map_err(|_| Failure::new(ErrorCode::InvalidSettings, ErrorPath::Effects))
}
