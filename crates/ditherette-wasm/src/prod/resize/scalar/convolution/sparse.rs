//! Compact input for severe fixed-support shrink, preserving the original direct kernel.

use super::{kernel, ConvolutionResizePlan, SupportPolicy};
use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::contract::failure::Failure,
};

impl ConvolutionResizePlan {
    /// Store every original tap coordinate, including duplicate clamped edge taps.
    /// Each output coordinate owns `slots` entries; unused entries gather source pixel zero.
    pub(crate) fn write_sparse_offsets<'a>(
        &self,
        slots: usize,
        offsets: &'a mut [u8],
    ) -> (&'a [u8], &'a [u8]) {
        let column_bytes = self.output_dimensions().width_usize() * slots * 4;
        let (columns, rows) = offsets.split_at_mut(column_bytes);
        offsets_for_axis(&self.x_taps, slots, 4, columns);
        offsets_for_axis(
            &self.y_taps,
            slots,
            self.source_dimensions().width_usize() * 4,
            rows,
        );
        (columns, rows)
    }

    /// Only the scalar, no-callback, direct fixed-support path may enter here.
    /// Original geometry selects the existing kernel; the compact view supplies its stride.
    /// Gathering is already complete, so no JavaScript executes while indices are remapped.
    pub(crate) fn resize_sparse_fixed(
        &mut self,
        source: ImageView<'_, Rgba8>,
        output: ImageViewMut<'_, Rgba8>,
        slots: usize,
        columns: &[u8],
        rows: &[u8],
    ) -> Result<(), Failure> {
        assert_eq!(self.support_policy(), SupportPolicy::Fixed);
        assert!(!kernel::should_use_blocks(self) && !kernel::should_use_x_then_y(self));
        assert_eq!(
            source.dimensions().width_usize(),
            self.output_dimensions().width_usize() * slots
        );
        assert_eq!(
            source.dimensions().height_usize(),
            self.output_dimensions().height_usize() * slots
        );
        assert_eq!(output.dimensions(), self.output_dimensions());
        let guard = Remapped {
            source_stride: self.source_dimensions().width_usize() * 4,
            plan: self,
            slots,
            columns,
            rows,
        };
        for axis in [&mut guard.plan.x_taps, &mut guard.plan.y_taps] {
            for (coordinate, taps) in axis.iter_mut().enumerate() {
                for (slot, tap) in taps.iter_mut().enumerate() {
                    tap.index = coordinate * slots + slot;
                }
            }
        }
        kernel::resize_with_progress_known_opacity(
            source,
            output,
            guard.plan,
            None,
            false,
            &mut |_| Ok(()),
        )
    }
}

fn offsets_for_axis(
    axis: &[Vec<super::plan::AxisTap>],
    slots: usize,
    stride: usize,
    offsets: &mut [u8],
) {
    assert_eq!(offsets.len(), axis.len() * slots * 4);
    offsets.fill(0);
    for (coordinate, taps) in axis.iter().enumerate() {
        assert!(taps.len() <= slots);
        for (slot, tap) in taps.iter().enumerate() {
            let position = (coordinate * slots + slot) * 4;
            offsets[position..position + 4]
                .copy_from_slice(&((tap.index * stride) as u32).to_le_bytes());
        }
    }
}

/// A cached plan must be restored even if a native caller unwinds during execution.
struct Remapped<'a> {
    plan: &'a mut ConvolutionResizePlan,
    slots: usize,
    source_stride: usize,
    columns: &'a [u8],
    rows: &'a [u8],
}

impl Drop for Remapped<'_> {
    fn drop(&mut self) {
        for (axis, offsets, stride) in [
            (&mut self.plan.x_taps, self.columns, 4),
            (&mut self.plan.y_taps, self.rows, self.source_stride),
        ] {
            for (coordinate, taps) in axis.iter_mut().enumerate() {
                for (slot, tap) in taps.iter_mut().enumerate() {
                    let position = (coordinate * self.slots + slot) * 4;
                    tap.index =
                        u32::from_le_bytes(offsets[position..position + 4].try_into().unwrap())
                            as usize
                            / stride;
                }
            }
        }
    }
}
