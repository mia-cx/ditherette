//! Shared result storage. Semantic preparation belongs in spec/prod pipeline modules.

use serde::{Deserialize, Serialize};

use super::{ImageBuf, PaletteIndex8, Rgba8};

/// A caller-selected entry. Position defines its stable output index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PaletteEntry {
    Color { rgb: [u8; 3] },
    Transparent,
}

/// Packed RGBA palette entries in retained input order, including duplicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedPalette {
    pub rgba: Vec<u8>,
    pub transparent_index: Option<u8>,
}

/// Stable warning identifiers; messages retain the website's approved wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningCode {
    PaletteTruncated,
    TransparentOnly,
    TransparentFallback,
}

/// A caller-visible diagnostic, compared together with pixels during conformance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessWarning {
    pub code: WarningCode,
    pub message: String,
}

/// Durable owned RGBA8 stage output. The JS boundary copies its bytes to JS ownership.
pub type Rgba8Image = ImageBuf<Rgba8>;

/// Owned indexed pixels plus the exact palette and warnings needed to interpret them.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedImage {
    pub indices: ImageBuf<PaletteIndex8>,
    pub palette: NormalizedPalette,
    pub warnings: Vec<ProcessWarning>,
}
