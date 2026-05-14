use std::fmt;

/// Errors produced by validation or image-processing steps.
///
/// Boundary adapters convert this type into JavaScript-facing errors. Internal
/// modules keep this Rust-native error so processing code does not depend on
/// `wasm-bindgen` types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingError {
    /// A required width or height was zero.
    ZeroDimension {
        /// Name of the dimension that was zero.
        name: &'static str,
    },
    /// Checked arithmetic overflowed while calculating a pixel or byte count.
    SizeOverflow {
        /// Operation or value whose checked size calculation overflowed.
        context: &'static str,
    },
    /// A caller-supplied processing parameter was outside the supported range.
    InvalidParameter {
        /// Name of the invalid parameter.
        name: &'static str,
        /// Human-readable constraint that the parameter violated.
        reason: &'static str,
    },
    /// A byte buffer did not match the dimensions that describe it.
    InvalidBufferLength {
        /// Required byte length for the supplied dimensions.
        expected: usize,
        /// Actual byte length supplied by the caller.
        actual: usize,
    },
}

impl fmt::Display for ProcessingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension { name } => write!(formatter, "{name} must be greater than zero"),
            Self::SizeOverflow { context } => write!(formatter, "{context} is too large"),
            Self::InvalidParameter { name, reason } => write!(formatter, "{name} {reason}"),
            Self::InvalidBufferLength { expected, actual } => write!(
                formatter,
                "RGBA buffer length mismatch: expected {expected} bytes, got {actual} bytes"
            ),
        }
    }
}

impl std::error::Error for ProcessingError {}
