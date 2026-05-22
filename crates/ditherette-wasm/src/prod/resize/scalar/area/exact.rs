//! Exact integer-scale area fast-path routing.
//!
//! This module only decides whether an area resize is an exact integer scale and
//! dispatches to the relevant exact kernel. Fractional planned coverage lives in
//! `planned`; exact downscale arithmetic lives in `downscale`.

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub(super) fn resize_exact_integer_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    let Some((x_step, y_step)) = exact_integer_downscale_steps(source, output) else {
        return false;
    };

    super::downscale::resize_exact_downscale_into(source, output, x_step, y_step);
    true
}

pub(super) fn resize_exact_integer_upscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    let Some((x_step, y_step)) = exact_integer_upscale_steps(source, output) else {
        return false;
    };

    let output_dimensions = output.dimensions();
    common::rgba8::resize_exact_pixel_repeat_into(
        source.data(),
        source.dimensions(),
        output.data_mut(),
        output_dimensions,
        x_step,
        y_step,
    );
    true
}

fn exact_integer_downscale_steps(
    source: ImageView<'_, Rgba8>,
    output: &ImageViewMut<'_, Rgba8>,
) -> Option<(usize, usize)> {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let source_width = source_dimensions.width();
    let source_height = source_dimensions.height();
    let output_width = output_dimensions.width();
    let output_height = output_dimensions.height();

    if source_width < output_width || source_height < output_height {
        return None;
    }
    if source_width == output_width && source_height == output_height {
        return None;
    }
    if !source_width.is_multiple_of(output_width) || !source_height.is_multiple_of(output_height) {
        return None;
    }

    Some((
        (source_width / output_width) as usize,
        (source_height / output_height) as usize,
    ))
}

fn exact_integer_upscale_steps(
    source: ImageView<'_, Rgba8>,
    output: &ImageViewMut<'_, Rgba8>,
) -> Option<(usize, usize)> {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let source_width = source_dimensions.width();
    let source_height = source_dimensions.height();
    let output_width = output_dimensions.width();
    let output_height = output_dimensions.height();

    if output_width < source_width || output_height < source_height {
        return None;
    }
    if output_width == source_width && output_height == source_height {
        return None;
    }
    if !output_width.is_multiple_of(source_width) || !output_height.is_multiple_of(source_height) {
        return None;
    }

    Some((
        (output_width / source_width) as usize,
        (output_height / source_height) as usize,
    ))
}
