//! Readable output partitions, worker assignments, and sequential execution.

pub mod contract;
pub mod execution;

pub use contract::{RowBand, RowBandPlan};
pub use execution::{
    for_each_row_band, for_each_tile, RowBandWorkAssignment, RowBandWorkPlan, Tile, TileGrid,
    WorkerBudget, MAX_WORKER_BUDGET,
};
