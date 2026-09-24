//! Per-axis contribution plans for the exact trilinear stages.
//!
//! Both exact stages evaluate a separable 2D sum per output pixel. The x and y
//! contributions depend only on geometry, so each stage plans them once per axis
//! instead of recomputing them per output pixel. Kernels still visit y taps
//! outer and x taps inner and form each 2D weight exactly as before.

use std::mem::size_of;

use crate::prod::{contract::failure::Failure, resize::common::allocation::CapacityBudget};

/// One nonzero contribution: clamped source index and its axis weight or overlap.
#[derive(Debug, Clone, Copy, Default)]
pub struct AxisTap {
    pub index: u32,
    pub weight: f64,
}

/// Planned taps for every output coordinate of one axis, in evaluation order.
/// Storage is owned so repeated planning within reserved capacity never allocates.
#[derive(Debug, Default)]
pub struct AxisPlan {
    starts: Vec<u32>,
    taps: Vec<AxisTap>,
}

impl AxisPlan {
    /// Heap bytes reserved for `outputs` coordinates and `taps` contributions.
    pub fn required_bytes(outputs: usize, taps: usize) -> u64 {
        ((outputs + 1) * size_of::<u32>() + taps * size_of::<AxisTap>()) as u64
    }

    /// Reserve through the caller's ledger before any source bytes are read.
    pub fn try_reserve(
        budget: &mut CapacityBudget,
        outputs: usize,
        taps: usize,
    ) -> Result<Self, Failure> {
        Ok(Self {
            starts: budget.vector(outputs + 1)?,
            taps: budget.vector(taps)?,
        })
    }

    /// Replace the plan with `taps_for(output)` for each output coordinate.
    pub fn fill<I: Iterator<Item = AxisTap>>(
        &mut self,
        output_len: u32,
        taps_for: impl Fn(u32) -> I,
    ) {
        let reserved = (self.starts.capacity(), self.taps.capacity());
        self.starts.clear();
        self.taps.clear();
        self.starts.push(0);
        for output in 0..output_len {
            self.taps.extend(taps_for(output));
            self.starts.push(self.taps.len() as u32);
        }
        debug_assert!(
            reserved == (0, 0) || reserved == (self.starts.capacity(), self.taps.capacity()),
            "prepared plans must stay within their reserved capacity"
        );
    }

    /// Taps for one output coordinate.
    pub fn taps(&self, output: usize) -> &[AxisTap] {
        &self.taps[self.starts[output] as usize..self.starts[output + 1] as usize]
    }
}

/// Upper bound on area taps per output: `ceil(end) - floor(start) < scale + 2`.
pub fn area_taps_per_output(source_len: u32, output_len: u32) -> usize {
    (f64::from(source_len) / f64::from(output_len)).ceil() as usize + 1
}

/// Upper bound on triangle taps per output: `ceil(p + s) - floor(p - s) + 1 < 2s + 3`.
pub fn bilinear_taps_per_output(source_len: u32, output_len: u32) -> usize {
    ((f64::from(source_len) / f64::from(output_len)).max(1.0) * 2.0).ceil() as usize + 2
}
