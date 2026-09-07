//! Structured reference errors with stable codes and request paths.

use serde::{Deserialize, Serialize};

/// Error categories shared by all five methods and initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidImage,
    InvalidPalette,
    InvalidSettings,
    UnsupportedOperation,
    Capability,
    Initialization,
    MemoryLimit,
    WasmMemoryUnavailable,
    Disposed,
    ReentrantCall,
    Callback,
    Runtime,
}

/// Failed calls expose this diagnostic and no partial image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DitheretteError {
    pub code: ErrorCode,
    pub path: String,
    pub message: String,
}

impl DitheretteError {
    /// Constructs a stable category/path with a human-readable explanation.
    pub fn new(code: ErrorCode, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DitheretteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for DitheretteError {}
