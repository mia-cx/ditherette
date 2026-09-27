//! Complete processing through the existing borrowed input and caught indexed sink.

use super::{
    effects::decode,
    fields::parse_dither,
    processor::{
        dimension, parse_resize, restore_ready, status, take_ready, validate_resize_controls,
        JsBoundary,
    },
    quantize::{parse_alpha, parse_matching, read_palette, PALETTE_SLOTS},
};
use crate::{
    image::contracts::PaletteEntry,
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{Output, RecipeV1, MAX_OUTPUT_SIDE, MAX_SOURCE_SIDE},
        },
        pipeline::process::ProcessRequest,
    },
};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

/// Existing resize, matching, alpha, and dither tags retain their staged encodings.
/// Every control is checked as f64 before narrowing. Only final indexed output is published.
#[wasm_bindgen(js_name = privateProcess)]
pub fn private_process(
    input: &Uint8Array,
    source_width: f64,
    source_height: f64,
    palette: &JsValue,
    version: f64,
    output_width: f64,
    output_height: f64,
    algorithm: f64,
    anchor: f64,
    support: f64,
    matching: f64,
    alpha_mode: f64,
    alpha_threshold: f64,
    matte: f64,
    family: f64,
    field: f64,
    parameter: f64,
    space: f64,
    strength: f64,
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
    result_sink: &JsValue,
) -> u32 {
    process_call(
        input,
        source_width,
        source_height,
        palette,
        version,
        [output_width, output_height, algorithm, anchor, support],
        [matching, alpha_mode, alpha_threshold, matte],
        [
            family, field, parameter, space, strength, placement, radius, threshold, softness,
        ],
        None,
        result_sink,
    )
}

/// Recipe v2: the same terminal encoding plus the wrapper's normalized effects JSON.
/// Effects run in Wasm before resize; only the final indexed output is published.
#[wasm_bindgen(js_name = privateProcessEffects)]
pub fn private_process_effects(
    input: &Uint8Array,
    source_width: f64,
    source_height: f64,
    palette: &JsValue,
    version: f64,
    output_width: f64,
    output_height: f64,
    algorithm: f64,
    anchor: f64,
    support: f64,
    matching: f64,
    alpha_mode: f64,
    alpha_threshold: f64,
    matte: f64,
    family: f64,
    field: f64,
    parameter: f64,
    space: f64,
    strength: f64,
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
    effects: &str,
    result_sink: &JsValue,
) -> u32 {
    process_call(
        input,
        source_width,
        source_height,
        palette,
        version,
        [output_width, output_height, algorithm, anchor, support],
        [matching, alpha_mode, alpha_threshold, matte],
        [
            family, field, parameter, space, strength, placement, radius, threshold, softness,
        ],
        Some(effects),
        result_sink,
    )
}

#[allow(clippy::too_many_arguments)]
fn process_call(
    input: &Uint8Array,
    source_width: f64,
    source_height: f64,
    palette: &JsValue,
    version: f64,
    [output_width, output_height, algorithm, anchor, support]: [f64; 5],
    [matching, alpha_mode, alpha_threshold, matte]: [f64; 4],
    [family, field, parameter, space, strength, placement, radius, threshold, softness]: [f64; 9],
    effects: Option<&str>,
    result_sink: &JsValue,
) -> u32 {
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        let expected = if effects.is_some() { 2.0 } else { 1.0 };
        if version != expected {
            return Err(Failure::new(
                ErrorCode::InvalidRequest,
                ErrorPath::RecipeVersion,
            ));
        }
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
        validate_resize_controls(algorithm, anchor, support)?;
        let recipe = RecipeV1 {
            version: 1,
            output: Output {
                width: dimension(
                    output_width,
                    MAX_OUTPUT_SIDE,
                    ErrorCode::InvalidSettings,
                    ErrorPath::OutputWidth,
                )?,
                height: dimension(
                    output_height,
                    MAX_OUTPUT_SIDE,
                    ErrorCode::InvalidSettings,
                    ErrorPath::OutputHeight,
                )?,
                resize: parse_resize(algorithm, anchor, support)?,
            },
            matching: parse_matching(matching)?,
            alpha: parse_alpha(alpha_mode, alpha_threshold, matte)?,
            dither: parse_dither(
                family, field, parameter, space, strength, placement, radius, threshold, softness,
            )?,
        };
        let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
        let count = read_palette(palette, &mut entries)?;
        let request = ProcessRequest {
            source_width,
            source_height,
            palette: &entries[..count],
            recipe,
        };
        let mut boundary = JsBoundary::new(input, result_sink)?;
        match effects {
            Some(effects) => processor.process_effects(request, &decode(effects)?, &mut boundary),
            None => processor.process(request, &mut boundary),
        }
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}
