//! Executable specs for mapping color-coordinate images to palette indices.
//!
//! Color conversion lives under `spec::color`. This module owns the next
//! semantic step: given input coordinates and palette coordinates, choose the
//! nearest palette entry with a metric-specific distance function. Ties are
//! resolved by stable palette order because the scan only replaces the current
//! best index on strictly smaller distances.

pub mod metric;
pub mod nearest_color;
