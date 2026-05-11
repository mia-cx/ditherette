use std::fmt;

/// Errors produced by validation or image-processing steps.
///
/// Boundary adapters convert this type into JavaScript-facing errors. Internal
/// modules keep this Rust-native error so processing code does not depend on
/// `wasm-bindgen` types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingError {
    /// A required width or height was zero.
    ZeroDimension { name: &'static str },
    /// Checked arithmetic overflowed while calculating a pixel or byte count.
    SizeOverflow { context: &'static str },
    /// A byte buffer did not match the dimensions that describe it.
    InvalidBufferLength { expected: usize, actual: usize },
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension { name } => write!(formatter, "{name} must be greater than zero"),
            Self::SizeOverflow { context } => write!(formatter, "{context} is too large"),
            Self::InvalidBufferLength { expected, actual } => write!(
                formatter,
                "RGBA buffer length mismatch: expected {expected} bytes, got {actual} bytes"
            ),
        }
    }
}

impl std::error::Error for ProcessingError {}
