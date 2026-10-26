//! Ditherette as an After Effects effect.
//!
//! [`render`] and [`look`] hold everything testable on any OS. The `plugin`
//! module binds them to the After Effects SDK, which only builds on macOS and Windows.

pub mod look;
pub mod palette;
pub mod render;

#[cfg(any(target_os = "macos", target_os = "windows"))]
mod plugin;
