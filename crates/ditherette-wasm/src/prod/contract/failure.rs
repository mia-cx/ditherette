//! Allocation-free private failures. Public adapters supply human-readable text.

use super::error::ErrorCode;

/// Stable numeric private ABI paths. These are not public recipe settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorPath {
    Instance = 0,
    MemoryLimitBytes = 1,
    SourceWidth = 2,
    SourceHeight = 3,
    SourceData = 4,
    Source = 5,
    OutputWidth = 6,
    OutputHeight = 7,
    Output = 8,
    OutputAnchor = 9,
    Wasm = 10,
    Control = 11,
    OutputResize = 12,
    Palette = 13,
    Alpha = 14,
    AlphaThreshold = 15,
    Matching = 16,
    Perturb = 17,
    PerturbField = 18,
    PerturbSpace = 19,
    PerturbStrength = 20,
    PerturbPlacement = 21,
    PerturbRadius = 22,
    PerturbThreshold = 23,
    PerturbSoftness = 24,
    Dither = 25,
    DitherPlacement = 27,
    DitherRadius = 28,
    DitherThreshold = 29,
    DitherSoftness = 30,
    DitherSize = 36,
}

/// A small value usable even when Rust cannot allocate an error string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Failure {
    pub code: ErrorCode,
    pub path: ErrorPath,
}

impl Failure {
    pub const fn new(code: ErrorCode, path: ErrorPath) -> Self {
        Self { code, path }
    }

    /// Stable private status. Zero is reserved for success.
    pub const fn status(self) -> u32 {
        match self.code {
            ErrorCode::InvalidRequest => 1,
            ErrorCode::InvalidImage => 2,
            ErrorCode::InvalidPalette => 3,
            ErrorCode::InvalidSettings => 4,
            ErrorCode::UnsupportedOperation => 5,
            ErrorCode::Capability => 6,
            ErrorCode::Initialization => 7,
            ErrorCode::MemoryLimit => 8,
            ErrorCode::WasmMemoryUnavailable => 9,
            ErrorCode::Disposed => 10,
            ErrorCode::ReentrantCall => 11,
            ErrorCode::Callback => 12,
            ErrorCode::Runtime => 13,
        }
    }
}
