//! Naive scheduler-neutral geometry and assignment models.
//! Workers own consecutive bands. Visitors run sequentially and stop at the first error.

use crate::image::ImageDimensions;

use super::{RowBand, RowBandPlan};

/// A nonempty half-open rectangle in complete-output coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    x_start: u32,
    y_start: u32,
    x_end: u32,
    y_end: u32,
}

impl Tile {
    /// Rejects empty or reversed rectangles.
    pub fn new(x_start: u32, y_start: u32, x_end: u32, y_end: u32) -> Option<Self> {
        (x_start < x_end && y_start < y_end).then_some(Self {
            x_start,
            y_start,
            x_end,
            y_end,
        })
    }

    /// First included column.
    pub const fn x_start(self) -> u32 {
        self.x_start
    }
    /// First included row.
    pub const fn y_start(self) -> u32 {
        self.y_start
    }
    /// First excluded column.
    pub const fn x_end(self) -> u32 {
        self.x_end
    }
    /// First excluded row.
    pub const fn y_end(self) -> u32 {
        self.y_end
    }
    /// Number of included columns.
    pub const fn width(self) -> u32 {
        self.x_end - self.x_start
    }
    /// Number of included rows.
    pub const fn height(self) -> u32 {
        self.y_end - self.y_start
    }
}

/// A row-major collection of disjoint rectangles covering the complete output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileGrid {
    output_dimensions: ImageDimensions,
    tiles: Vec<Tile>,
}

impl TileGrid {
    /// Caps each rectangle by the requested size and the remaining output extent.
    pub fn new(dimensions: ImageDimensions, width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            return None;
        }
        let mut tiles = Vec::new();
        let mut y = 0;
        while y < dimensions.height() {
            let bottom = y + height.min(dimensions.height() - y);
            let mut x = 0;
            while x < dimensions.width() {
                let right = x + width.min(dimensions.width() - x);
                tiles.push(Tile::new(x, y, right, bottom)?);
                x = right;
            }
            y = bottom;
        }
        Some(Self {
            output_dimensions: dimensions,
            tiles,
        })
    }

    /// Complete output dimensions, independent of individual rectangles.
    pub const fn output_dimensions(&self) -> ImageDimensions {
        self.output_dimensions
    }
    /// Rectangles in row-major visitation order.
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
}

/// Existing execution policy caps pool capacity at eight workers.
pub const MAX_WORKER_BUDGET: u32 = 8;

/// Nonzero pool capacity, independent of any image operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerBudget {
    pool_size: u32,
}

impl WorkerBudget {
    /// Clamps explicit pool capacity to one through eight.
    pub fn new(pool_size: u32) -> Self {
        Self {
            pool_size: pool_size.clamp(1, MAX_WORKER_BUDGET),
        }
    }
    /// Uses half the logical CPUs, then applies the same capacity bounds.
    pub fn from_available_parallelism(cpus: u32) -> Self {
        Self::new(cpus / 2)
    }
    /// Maximum workers retained by the pool.
    pub const fn pool_size(self) -> u32 {
        self.pool_size
    }
    /// Every distinct nonzero count within capacity.
    pub fn worker_counts(self) -> impl Iterator<Item = u32> {
        1..=self.pool_size
    }
    /// Caps requested workers by capacity and available work, allowing zero only for no work.
    pub fn active_workers(self, requested: u32, work_items: u32) -> u32 {
        requested.max(1).min(self.pool_size).min(work_items)
    }
    /// Whether a requested count needs no clamping and has work for every worker.
    pub fn can_use_workers(self, requested: u32, work_items: u32) -> bool {
        requested != 0 && requested <= self.pool_size && requested <= work_items
    }
}

/// One worker's consecutive bands, retaining complete-output row coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowBandWorkAssignment {
    worker_index: u32,
    bands: Vec<RowBand>,
}

impl RowBandWorkAssignment {
    /// Zero-based active worker number.
    pub const fn worker_index(&self) -> u32 {
        self.worker_index
    }
    /// Consecutive bands assigned to this worker.
    pub fn bands(&self) -> &[RowBand] {
        &self.bands
    }
}

/// Balanced consecutive chunks, with extra bands assigned to earlier workers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowBandWorkPlan {
    requested_workers: u32,
    active_workers: u32,
    assignments: Vec<RowBandWorkAssignment>,
}

impl RowBandWorkPlan {
    /// Divides band count by active worker count, preserving original band order.
    pub fn new(plan: &RowBandPlan, budget: WorkerBudget, requested: u32) -> Option<Self> {
        let count = u32::try_from(plan.bands().len()).ok()?;
        let workers = budget.active_workers(requested, count);
        if workers == 0 {
            return None;
        }
        let mut assignments = Vec::new();
        let mut cursor = 0;
        for worker in 0..workers {
            let size = count / workers + u32::from(worker < count % workers);
            let mut bands = Vec::new();
            for _ in 0..size {
                bands.push(plan.bands()[cursor]);
                cursor += 1;
            }
            assignments.push(RowBandWorkAssignment {
                worker_index: worker,
                bands,
            });
        }
        Some(Self {
            requested_workers: requested,
            active_workers: workers,
            assignments,
        })
    }
    /// Original request before capping.
    pub const fn requested_workers(&self) -> u32 {
        self.requested_workers
    }
    /// Number of workers receiving nonempty work.
    pub const fn active_workers(&self) -> u32 {
        self.active_workers
    }
    /// Assignments in worker order.
    pub fn assignments(&self) -> &[RowBandWorkAssignment] {
        &self.assignments
    }
}

/// Applies the callback in row order, returning its first error unchanged.
pub fn for_each_row_band<E>(
    plan: &RowBandPlan,
    mut visit: impl FnMut(RowBand) -> Result<(), E>,
) -> Result<(), E> {
    for &band in plan.bands() {
        visit(band)?;
    }
    Ok(())
}

/// Applies the callback in row-major tile order, returning its first error unchanged.
pub fn for_each_tile<E>(
    grid: &TileGrid,
    mut visit: impl FnMut(Tile) -> Result<(), E>,
) -> Result<(), E> {
    for &tile in grid.tiles() {
        visit(tile)?;
    }
    Ok(())
}
