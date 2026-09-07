//! Exact integer-scale area fast-path routing.
//!
//! This module only decides whether an area resize is an exact integer scale and
//! dispatches to the relevant exact kernel. Fractional planned coverage lives in
//! `planned`; exact downscale arithmetic lives in `downscale`.

use crate::{
    image::{rgba8, ImageDimensions, ImageView, ImageViewMut, Rgba8},
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

pub(super) fn resize_exact_integer_downscale_rows_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    y_start: u32,
) -> bool {
    let Some((x_step, y_step)) =
        exact_integer_downscale_steps_for_dimensions(source.dimensions(), full_output_dimensions)
    else {
        return false;
    };

    super::downscale::resize_exact_downscale_rows_into(source, output, x_step, y_step, y_start);
    true
}

pub(super) fn resize_exact_integer_upscale_rows_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    y_start: u32,
) -> bool {
    let Some((x_step, y_step)) =
        exact_integer_upscale_steps_for_dimensions(source.dimensions(), full_output_dimensions)
    else {
        return false;
    };

    resize_exact_pixel_repeat_rows_into(
        source,
        output,
        full_output_dimensions,
        x_step,
        y_step,
        y_start,
    );
    true
}

fn resize_exact_pixel_repeat_rows_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    x_factor: usize,
    y_factor: usize,
    y_start: u32,
) {
    let source_row_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_len = full_output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
    let source_width = source.dimensions().width_usize();

    for (local_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_len)
        .enumerate()
    {
        let output_y = y_start as usize + local_y;
        let source_y = output_y / y_factor;
        let source_row_start = source_y * source_row_len;

        for source_x in 0..source_width {
            let source_start = source_row_start + source_x * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source.data()[source_start..source_start + rgba8::RGBA8_CHANNELS];
            let output_x_start = source_x * x_factor * rgba8::RGBA8_CHANNELS;
            for repeat_x in 0..x_factor {
                let output_start = output_x_start + repeat_x * rgba8::RGBA8_CHANNELS;
                output_row[output_start..output_start + rgba8::RGBA8_CHANNELS]
                    .copy_from_slice(source_pixel);
            }
        }
    }
}

fn exact_integer_downscale_steps(
    source: ImageView<'_, Rgba8>,
    output: &ImageViewMut<'_, Rgba8>,
) -> Option<(usize, usize)> {
    exact_integer_downscale_steps_for_dimensions(source.dimensions(), output.dimensions())
}

pub(super) fn exact_integer_downscale_steps_for_dimensions(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Option<(usize, usize)> {
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
    if source_width % output_width != 0 || source_height % output_height != 0 {
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
    exact_integer_upscale_steps_for_dimensions(source.dimensions(), output.dimensions())
}

pub(super) fn exact_integer_upscale_steps_for_dimensions(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Option<(usize, usize)> {
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
    if output_width % source_width != 0 || output_height % source_height != 0 {
        return None;
    }

    Some((
        (output_width / source_width) as usize,
        (output_height / source_height) as usize,
    ))
}
