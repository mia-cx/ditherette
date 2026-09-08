//! Complete processing through the existing borrowed input and caught indexed sink.

use super::{
    fields::parse_dither,
    processor::{
        dimension, parse_resize, restore_ready, status, take_ready, validate_resize_controls,
    },
    quantize::{parse_alpha, parse_matching, read_palette, JsQuantizeBoundary, PALETTE_SLOTS},
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
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        if version != 1.0 {
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
        processor.process(
            ProcessRequest {
                source_width,
                source_height,
                palette: &entries[..count],
                recipe,
            },
            &mut JsQuantizeBoundary::new(input, result_sink)?,
        )
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}
