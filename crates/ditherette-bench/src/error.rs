//! Error and exit-code handling for the benchmark CLI.

use std::{error::Error, fmt, process::ExitCode};

#[derive(Debug)]
pub(crate) enum BenchError {
    Config(String),
    Verify(String),
    Acceptance(String),
    Baseline(String),
    Runtime(String),
}

impl BenchError {
    pub(crate) fn io(error: std::io::Error) -> Self {
        Self::Runtime(error.to_string())
    }

    pub(crate) fn exit_code(&self) -> ExitCode {
        match self {
            Self::Acceptance(_) => ExitCode::from(1),
            Self::Config(_) => ExitCode::from(2),
            Self::Verify(_) => ExitCode::from(3),
            Self::Baseline(_) => ExitCode::from(4),
            Self::Runtime(_) => ExitCode::from(5),
        }
    }
}

impl fmt::Display for BenchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message)
            | Self::Verify(message)
            | Self::Acceptance(message)
            | Self::Baseline(message)
            | Self::Runtime(message) => message.fmt(formatter),
        }
    }
}

impl Error for BenchError {}
