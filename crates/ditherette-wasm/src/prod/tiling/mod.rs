//! Generic production tiling primitives.
//!
//! This module owns domain-agnostic output-space partitions used by production
//! resize, color conversion, dithering, and quantization implementations. It
//! describes geometry and sequential execution only; domain adapters own pixel
//! semantics, halo requirements, scratch storage, and benchmark subjects.

mod executor;
mod grid;
mod row_band;
mod tile;

pub use executor::{for_each_row_band, for_each_tile};
pub use grid::TileGrid;
pub use row_band::{RowBand, RowBandPlan};
pub use tile::Tile;
