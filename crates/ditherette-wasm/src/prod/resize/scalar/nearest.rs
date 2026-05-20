//! Scalar production nearest-neighbor resize.
//!
//! This starts as an intentionally duplicated implementation of nearest resize
//! semantics. It does not import spec code; future perf work can precompute maps
//! or specialize loops while preserving oracle compatibility.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::common::alignment::{map_axis_coordinate, ResizeAnchor},
};

/// Resize `source` into `output` by copying the nearest source pixel.
pub fn resize_nearest_into<F: ImageFormat>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let (x_alignment, y_alignment) = anchor.axes();

    for output_y in 0..output_dimensions.height() {
        let source_y = map_axis_coordinate(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
        );
        let source_row = source
            .row(source_y)
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for output_x in 0..output_dimensions.width() {
            let source_x = map_axis_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
                x_alignment,
            );
            let source_start = source_x as usize * F::CHANNEL_COUNT;
            let output_start = output_x as usize * F::CHANNEL_COUNT;
            let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];

            output_pixel.copy_from_slice(source_pixel);
        }
    }
}
