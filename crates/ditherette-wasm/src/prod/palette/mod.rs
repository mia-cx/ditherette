//! Ordered supplied-palette preparation and byte-alpha rules for indexed matching.
//!
//! Call `Request::validate` before preparing a palette. Standalone perturbation
//! keeps its source alpha and hidden RGB; it does not use indexed alpha policy.

pub(crate) mod allocation;
use allocation::Budget;
pub use allocation::PreparationError;
use std::mem::size_of;

use crate::image::{
    contracts::{IndexedImage, NormalizedPalette, PaletteEntry, ProcessWarning, WarningCode},
    ImageBuf, PaletteIndex8,
};

use super::contract::error::ErrorCode;
use super::contract::request::{AlphaPolicy, MAX_PALETTE_ENTRIES};

const TRUNCATED: &str = "Palette was truncated to 256 entries for indexed PNG export.";
const TRANSPARENT_ONLY: &str = "Only Transparent is enabled; every output pixel is transparent.";
const FALLBACK: &str =
    "Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color.";

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
    /// Validates palette/alpha settings and reserves all owned capacities fallibly.
    pub fn try_new(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        memory_limit: u64,
    ) -> Result<Self, PreparationError> {
        let required = Self::required_capacity_bytes(entries, alpha)?;
        if required > memory_limit {
            return Err(PreparationError::memory());
        }
        let mut budget = Budget::new(memory_limit, size_of::<Self>() as u64)?;
        Self::prepare(entries, alpha, &mut budget)
    }

    /// Record, Vec capacities and warning strings reserved by `try_new`.
    pub fn required_capacity_bytes(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
    ) -> Result<u64, PreparationError> {
        if entries.is_empty() {
            return Err(PreparationError {
                code: ErrorCode::InvalidPalette,
                path: "palette",
            });
        }
        if let AlphaPolicy::Preserve { threshold } = alpha {
            if !threshold.is_finite() || !(0.0..=255.0).contains(&threshold) {
                return Err(PreparationError {
                    code: ErrorCode::InvalidSettings,
                    path: "alpha.threshold",
                });
            }
        }
        let retained = &entries[..entries.len().min(MAX_PALETTE_ENTRIES)];
        let visible = retained
            .iter()
            .filter(|entry| matches!(entry, PaletteEntry::Color { .. }))
            .count();
        let warnings = warning_messages(entries, alpha);
        Ok((size_of::<Self>()
            + retained.len() * 4
            + visible * size_of::<VisibleColor>()
            + warnings
                .iter()
                .flatten()
                .map(|(_, text)| size_of::<ProcessWarning>() + text.len())
                .sum::<usize>()) as u64)
    }

    /// Actual owned record and heap capacities, including warning text.
    pub fn capacity_bytes(&self) -> u64 {
        (size_of::<Self>()
            + self.palette.rgba.capacity()
            + self.visible.capacity() * size_of::<VisibleColor>()
            + self.warnings.capacity() * size_of::<ProcessWarning>()
            + self
                .warnings
                .iter()
                .map(|warning| warning.message.capacity())
                .sum::<usize>()) as u64
    }

    pub(crate) fn prepare(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        budget: &mut Budget,
    ) -> Result<Self, PreparationError> {
        let retained = &entries[..entries.len().min(MAX_PALETTE_ENTRIES)];
        let mut palette = NormalizedPalette {
            rgba: Vec::new(),
            transparent_index: None,
        };
        let mut visible = Vec::new();
        let mut darkest: Option<(u16, u8)> = None;
        let mut warnings = Vec::new();
        let messages = warning_messages(entries, alpha);
        budget.reserve(&mut palette.rgba, retained.len() * 4)?;
        budget.reserve(
            &mut visible,
            retained
                .iter()
                .filter(|entry| matches!(entry, PaletteEntry::Color { .. }))
                .count(),
        )?;
        budget.reserve(&mut warnings, messages.iter().flatten().count())?;
        for (code, message) in messages.into_iter().flatten() {
            warnings.push(ProcessWarning {
                code,
                message: budget.string(message)?,
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

        let threshold_index = palette
            .transparent_index
            .or(darkest.map(|(_, index)| index))
            .expect("request validation requires a nonempty palette");
        Ok(Self {
            palette,
            visible,
            warnings,
            alpha,
            threshold_index,
        })
    }

    /// Byte cutoff and fallback for a row with preserved alpha. Validated fractional
    /// thresholds have the same cutoff as their floor because source alpha is integral.
    pub(crate) fn preserved_alpha(&self) -> Option<(u8, u8)> {
        match self.alpha {
            AlphaPolicy::Preserve { threshold } => Some((threshold as u8, self.threshold_index)),
            _ => None,
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
            AlphaPolicy::Premultiplied {} => {
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

fn warning_messages(
    entries: &[PaletteEntry],
    alpha: AlphaPolicy,
) -> [Option<(WarningCode, &'static str)>; 2] {
    let retained = &entries[..entries.len().min(MAX_PALETTE_ENTRIES)];
    let visible = retained
        .iter()
        .any(|entry| matches!(entry, PaletteEntry::Color { .. }));
    let transparent = retained
        .iter()
        .any(|entry| matches!(entry, PaletteEntry::Transparent {}));
    [
        (entries.len() > MAX_PALETTE_ENTRIES).then_some((WarningCode::PaletteTruncated, TRUNCATED)),
        if !visible {
            Some((WarningCode::TransparentOnly, TRANSPARENT_ONLY))
        } else if matches!(alpha, AlphaPolicy::Preserve { .. }) && !transparent {
            Some((WarningCode::TransparentFallback, FALLBACK))
        } else {
            None
        },
    ]
}
