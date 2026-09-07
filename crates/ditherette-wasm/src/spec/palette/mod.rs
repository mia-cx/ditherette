//! Ordered supplied-palette preparation and byte-alpha rules for indexed matching.
//!
//! Call `Request::validate` before preparing a palette. Standalone perturbation
//! keeps its source alpha and hidden RGB; it does not use indexed alpha policy.

use crate::image::{
    contracts::{IndexedImage, NormalizedPalette, PaletteEntry, ProcessWarning, WarningCode},
    ImageBuf, PaletteIndex8,
};

use super::contract::request::{AlphaPolicy, MAX_PALETTE_ENTRIES};

/// A visible RGB entry and its original index in the retained ordered palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleColor {
    pub index: u8,
    pub rgb: [u8; 3],
}

/// Either a fixed output index or byte RGB that still requires visible matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PalettePixel {
    /// Skip dithering and matching. Diffusion drops incoming error and emits none.
    Index(u8),
    Color([u8; 3]),
}

/// Readable palette state. Duplicate entries remain distinct and in caller order.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedPalette {
    pub palette: NormalizedPalette,
    pub visible: Vec<VisibleColor>,
    pub warnings: Vec<ProcessWarning>,
    alpha: AlphaPolicy,
    threshold_index: u8,
}

impl PreparedPalette {
    /// Prepares a nonempty, request-validated palette and alpha policy.
    /// At most the first 256 entries survive; their indices never change.
    pub fn new(entries: &[PaletteEntry], alpha: AlphaPolicy) -> Self {
        let retained = &entries[..entries.len().min(MAX_PALETTE_ENTRIES)];
        let mut palette = NormalizedPalette {
            rgba: Vec::new(),
            transparent_index: None,
        };
        let mut visible = Vec::new();
        let mut darkest: Option<(u16, u8)> = None;
        let mut warnings = Vec::new();
        if entries.len() > MAX_PALETTE_ENTRIES {
            warnings.push(ProcessWarning {
                code: WarningCode::PaletteTruncated,
                message: "Palette was truncated to 256 entries for indexed PNG export.".into(),
            });
        }

        for (index, entry) in retained.iter().enumerate() {
            let index = index as u8;
            match *entry {
                PaletteEntry::Transparent {} => {
                    palette.rgba.extend_from_slice(&[0, 0, 0, 0]);
                    if palette.transparent_index.is_none() {
                        palette.transparent_index = Some(index);
                    }
                }
                PaletteEntry::Color { rgb: [r, g, b] } => {
                    palette.rgba.extend_from_slice(&[r, g, b, 255]);
                    visible.push(VisibleColor {
                        index,
                        rgb: [r, g, b],
                    });
                    let sum = u16::from(r) + u16::from(g) + u16::from(b);
                    if darkest.map_or(true, |(best, _)| sum < best) {
                        darkest = Some((sum, index));
                    }
                }
            }
        }

        if visible.is_empty() {
            warnings.push(ProcessWarning {
                code: WarningCode::TransparentOnly,
                message: "Only Transparent is enabled; every output pixel is transparent.".into(),
            });
        } else if matches!(alpha, AlphaPolicy::Preserve { .. })
            && palette.transparent_index.is_none()
        {
            warnings.push(ProcessWarning {
                code: WarningCode::TransparentFallback,
                message: "Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color.".into(),
            });
        }

        let threshold_index = palette
            .transparent_index
            .or(darkest.map(|(_, index)| index))
            .expect("request validation requires a nonempty palette");
        Self {
            palette,
            visible,
            warnings,
            alpha,
            threshold_index,
        }
    }

    /// Applies the indexed-output alpha policy before color conversion/matching.
    /// Alpha comparison and compositing retain JavaScript's f64 precision.
    pub fn prepare_pixel(&self, [r, g, b, alpha]: [u8; 4]) -> PalettePixel {
        if self.visible.is_empty() {
            return PalettePixel::Index(self.threshold_index);
        }
        let opacity = f64::from(alpha) / 255.0;
        let rgb = match self.alpha {
            AlphaPolicy::Preserve { threshold } => {
                if f64::from(alpha) <= threshold {
                    return PalettePixel::Index(self.threshold_index);
                }
                [r, g, b]
            }
            AlphaPolicy::Premultiplied => {
                [r, g, b].map(|channel| (f64::from(channel) * opacity).round() as u8)
            }
            AlphaPolicy::Matte { rgb: matte } => {
                let background = 1.0 - opacity;
                std::array::from_fn(|channel| {
                    let source = f64::from([r, g, b][channel]);
                    let matte = f64::from(matte[channel]);
                    (source * opacity + matte * background)
                        .round()
                        .clamp(0.0, 255.0) as u8
                })
            }
        };
        PalettePixel::Color(rgb)
    }

    /// Moves normalized palette/warnings into a result after successful matching.
    pub fn into_indexed(self, indices: ImageBuf<PaletteIndex8>) -> IndexedImage {
        IndexedImage {
            indices,
            palette: self.palette,
            warnings: self.warnings,
        }
    }
}
