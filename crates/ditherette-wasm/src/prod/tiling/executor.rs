//! Sequential visitors and caller-budgeted pooled row batches.
//!
//! Domain callers choose tile sizes and active worker counts. Pooled execution
//! uses the already-initialized Rayon pool when threads are compiled; scalar
//! builds run the same assignments sequentially. No crossover policy lives here.

use super::{RowBand, RowBandPlan, RowBandWorkAssignment, RowBandWorkPlan, TileGrid};

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

/// Execute one band per active worker per batch using disjoint output and caller-owned scratch.
/// The caller reports progress after each joined batch; callbacks never run on pool workers.
/// A failed batch finishes all its workers before returning. This helper selects no crossover policy.
pub fn execute_row_band_work<T: Send, S: Send, E: Send>(
    work: &RowBandWorkPlan,
    output: &mut [T],
    row_elements: usize,
    scratch: &mut [S],
    render: &(impl Fn(RowBand, &mut [T], &mut S) -> Result<u64, E> + Sync),
    progress: &mut impl FnMut(u64) -> Result<(), E>,
) -> Result<(), E> {
    let assignments = work.assignments();
    assert_eq!(scratch.len(), assignments.len());
    assert_eq!(
        output.len(),
        assignments.last().unwrap().bands().last().unwrap().y_end() as usize * row_elements
    );
    let batches = assignments
        .iter()
        .map(|assignment| assignment.bands().len())
        .max()
        .unwrap();
    let mut completed = 0;
    for batch in 0..batches {
        completed += execute_batch(assignments, output, row_elements, scratch, batch, render)?;
        progress(completed)?;
    }
    Ok(())
}

fn execute_batch<T: Send, S: Send, E: Send>(
    assignments: &[RowBandWorkAssignment],
    output: &mut [T],
    row_elements: usize,
    scratch: &mut [S],
    batch: usize,
    render: &(impl Fn(RowBand, &mut [T], &mut S) -> Result<u64, E> + Sync),
) -> Result<u64, E> {
    let start = assignments[0].bands()[0].y_start();
    if assignments.len() == 1 {
        let Some(&band) = assignments[0].bands().get(batch) else {
            return Ok(0);
        };
        let first = (band.y_start() - start) as usize * row_elements;
        let last = (band.y_end() - start) as usize * row_elements;
        return render(band, &mut output[first..last], &mut scratch[0]);
    }
    let middle = assignments.len() / 2;
    let split = (assignments[middle].bands()[0].y_start() - start) as usize * row_elements;
    let (left_output, right_output) = output.split_at_mut(split);
    let (left_scratch, right_scratch) = scratch.split_at_mut(middle);
    let left = || {
        execute_batch(
            &assignments[..middle],
            left_output,
            row_elements,
            left_scratch,
            batch,
            render,
        )
    };
    let right = || {
        execute_batch(
            &assignments[middle..],
            right_output,
            row_elements,
            right_scratch,
            batch,
            render,
        )
    };
    #[cfg(feature = "threads")]
    let (left, right) = rayon::join(left, right);
    #[cfg(not(feature = "threads"))]
    let (left, right) = {
        let (mut left, mut right) = (left, right);
        (left(), right())
    };
    Ok(left? + right?)
}
