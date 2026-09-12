//! Generic production tiling primitives.
//!
//! This module owns domain-agnostic output-space partitions plus worker-budget
//! and work-assignment plumbing used by production resize, color conversion,
//! dithering, and quantization implementations. Domain adapters own pixel
//! semantics, halo requirements, scratch storage, and benchmark subjects.

mod executor;
mod grid;
mod policy;
mod row_band;
mod tile;
mod work;

pub use executor::{for_each_row_band, for_each_tile};
pub use grid::TileGrid;
pub use policy::{WorkerBudget, MAX_WORKER_BUDGET};
pub use row_band::{RowBand, RowBandPlan};
pub use tile::Tile;
pub use work::{RowBandWorkAssignment, RowBandWorkPlan};
