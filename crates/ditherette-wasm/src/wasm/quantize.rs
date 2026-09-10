//! Borrowed, caught quantize boundary sharing the scalar processor's lifecycle.

use super::processor::{
    copy_input, dimension, input_length, restore_ready, snapshot_input, status, take_ready,
};
use crate::{
    image::{
        contracts::{PaletteEntry, WarningCode},
        ImageDimensions,
    },
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{AlphaPolicy, MatchPolicy, MAX_SOURCE_SIDE},
        },
        pipeline::quantize::IndexedMetadataRef,
        pipeline::quantize::{QuantizeBoundary, QuantizeRequest},
    },
};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/wasm/quantize_helpers.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = paletteLength)]
    fn palette_length(palette: &JsValue) -> Result<f64, JsValue>;
    #[wasm_bindgen(catch, js_name = paletteEntry)]
    fn palette_entry(palette: &JsValue, index: u32) -> Result<f64, JsValue>;
    #[wasm_bindgen(catch, js_name = completeIndexedResult)]
    fn complete_indexed(
        indices: &[u8],
        width: u32,
        height: u32,
        palette: &[u8],
        transparent: f64,
        first_code: u32,
        first_message: &str,
        second_code: u32,
        second_message: &str,
        sink: &JsValue,
    ) -> Result<(), JsValue>;
}

pub(super) const PALETTE_SLOTS: usize = 257;
const TRANSPARENT: f64 = 16_777_216.0;

/// Compact palette codes are wrapper-owned RGB integers or the transparent sentinel.
/// At most 257 entries retain the distinction between a full and truncated input palette.
#[wasm_bindgen(js_name = privateQuantize)]
pub fn private_quantize(
    input: &Uint8Array,
    width: f64,
    height: f64,
    palette: &JsValue,
    matching: f64,
    alpha_mode: f64,
    threshold: f64,
    matte: f64,
    result_sink: &JsValue,
) -> u32 {
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        let width = dimension(
            width,
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceWidth,
        )?;
        let height = dimension(
            height,
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceHeight,
        )?;
        let matching = parse_matching(matching)?;
        let alpha = parse_alpha(alpha_mode, threshold, matte)?;
        let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
        let count = read_palette(palette, &mut entries)?;
        processor.quantize(
            QuantizeRequest {
                source_width: width,
                source_height: height,
                palette: &entries[..count],
                alpha,
                matching,
            },
            &mut JsQuantizeBoundary::new(input, result_sink)?,
        )
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}

pub(super) struct JsQuantizeBoundary<'a> {
    pub(super) input: &'a Uint8Array,
    pub(super) result_sink: &'a JsValue,
    progress: Option<super::progress::JsProgress<'a>>,
}
impl<'a> JsQuantizeBoundary<'a> {
    pub(super) fn new(input: &'a Uint8Array, result_sink: &'a JsValue) -> Result<Self, Failure> {
        Ok(Self {
            input,
            result_sink,
            progress: super::progress::JsProgress::new(result_sink)?,
        })
    }
}
impl QuantizeBoundary for JsQuantizeBoundary<'_> {
    type Output = ();
    fn progress(&mut self) -> Option<&mut dyn crate::prod::pipeline::progress::Callback> {
        self.progress
            .as_mut()
            .map(|progress| progress as &mut dyn crate::prod::pipeline::progress::Callback)
    }
    fn capacity_bytes(&self) -> u64 {
        (std::mem::size_of::<[PaletteEntry; PALETTE_SLOTS]>() + std::mem::size_of::<Self>()) as u64
    }
    fn input_len(&mut self) -> Result<usize, Failure> {
        input_length(self.input)
            .map(|n| n as usize)
            .map_err(|_| Failure::new(ErrorCode::InvalidImage, ErrorPath::SourceData))
    }
    fn copy_input(&mut self, destination: &mut [u8]) -> Result<(), Failure> {
        copy_input(destination, self.input)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::SourceData))
    }
    fn snapshot_input(&mut self, destination: &mut [u8], compare: bool) -> Result<bool, Failure> {
        snapshot_input(destination, self.input, compare)
            .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::SourceData))
    }
    fn complete(
        &mut self,
        indices: &[u8],
        dimensions: ImageDimensions,
        prepared: IndexedMetadataRef<'_>,
    ) -> Result<(), Failure> {
        let warning = |index: usize| {
            prepared.warnings.get(index).map_or((0, ""), |warning| {
                (
                    match warning.code {
                        WarningCode::PaletteTruncated => 1,
                        WarningCode::TransparentOnly => 2,
                        WarningCode::TransparentFallback => 3,
                    },
                    warning.message.as_str(),
                )
            })
        };
        let (first_code, first_message) = warning(0);
        let (second_code, second_message) = warning(1);
        complete_indexed(
            indices,
            dimensions.width(),
            dimensions.height(),
            &prepared.palette.rgba,
            prepared.palette.transparent_index.map_or(-1.0, f64::from),
            first_code,
            first_message,
            second_code,
            second_message,
            self.result_sink,
        )
        .map_err(|_| Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Output))
    }
}
fn invalid_palette() -> Failure {
    Failure::new(ErrorCode::InvalidPalette, ErrorPath::Palette)
}
fn rgb(value: f64) -> Option<[u8; 3]> {
    if !value.is_finite() || value.fract() != 0.0 || !(0.0..TRANSPARENT).contains(&value) {
        return None;
    }
    let value = value as u32;
    Some([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}

/// Shared private numeric matching tags; no public settings or allocation are introduced.
pub(super) fn parse_matching(matching: f64) -> Result<MatchPolicy, Failure> {
    Ok(match matching {
        0.0 => MatchPolicy::SrgbEuclidean,
        1.0 => MatchPolicy::LinearRgbEuclidean,
        2.0 => MatchPolicy::OklabEuclidean,
        3.0 => MatchPolicy::CielabEuclidean,
        4.0 => MatchPolicy::YcbcrEuclidean,
        5.0 => MatchPolicy::SrgbCompuphase,
        6.0 => MatchPolicy::SrgbRec601,
        7.0 => MatchPolicy::SrgbRec709,
        8.0 => MatchPolicy::OklchEuclidean,
        9.0 => MatchPolicy::OklchCircularHue,
        10.0 => MatchPolicy::OklchHueArc,
        11.0 => MatchPolicy::CielabCiede2000,
        12.0 => MatchPolicy::CielchEuclidean,
        13.0 => MatchPolicy::CielchCircularHue,
        14.0 => MatchPolicy::CielchHueArc,
        _ => {
            return Err(Failure::new(
                ErrorCode::UnsupportedOperation,
                ErrorPath::Matching,
            ))
        }
    })
}

pub(super) fn parse_alpha(
    alpha_mode: f64,
    threshold: f64,
    matte: f64,
) -> Result<AlphaPolicy, Failure> {
    Ok(match alpha_mode {
        0.0 if matte == 0.0 => AlphaPolicy::Preserve { threshold },
        1.0 if threshold == 0.0 && matte == 0.0 => AlphaPolicy::Premultiplied {},
        2.0 if threshold == 0.0 => AlphaPolicy::Matte {
            rgb: rgb(matte).ok_or(Failure::new(ErrorCode::InvalidSettings, ErrorPath::Alpha))?,
        },
        _ => return Err(Failure::new(ErrorCode::InvalidSettings, ErrorPath::Alpha)),
    })
}

pub(super) fn read_palette(
    palette: &JsValue,
    entries: &mut [PaletteEntry; PALETTE_SLOTS],
) -> Result<usize, Failure> {
    let count = palette_length(palette).map_err(|_| invalid_palette())?;
    if !count.is_finite() || count.fract() != 0.0 || !(1.0..=PALETTE_SLOTS as f64).contains(&count)
    {
        return Err(invalid_palette());
    }
    for (index, entry) in entries[..count as usize].iter_mut().enumerate() {
        let value = palette_entry(palette, index as u32).map_err(|_| invalid_palette())?;
        *entry = if value == TRANSPARENT {
            PaletteEntry::Transparent {}
        } else {
            PaletteEntry::Color {
                rgb: rgb(value).ok_or_else(invalid_palette)?,
            }
        };
    }
    Ok(count as usize)
}
