//! Sequential tiling executors.
//!
//! These helpers intentionally do not choose tile sizes or parallelism. They are
//! the small shared loop domain adapters can call before specialized schedulers
//! exist.

use super::{RowBandPlan, TileGrid};

/// Visits each row band in plan order.
pub fn for_each_row_band<E>(
    plan: &RowBandPlan,
    mut visit: impl FnMut(super::RowBand) -> Result<(), E>,
) -> Result<(), E> {
    for &band in plan.bands() {
        visit(band)?;
    }
    Ok(())
}

/// Visits each tile in row-major plan order.
pub fn for_each_tile<E>(
    grid: &TileGrid,
    mut visit: impl FnMut(super::Tile) -> Result<(), E>,
) -> Result<(), E> {
    for &tile in grid.tiles() {
        visit(tile)?;
    }
    Ok(())
}
