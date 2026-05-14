use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::validate_resize_buffers,
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct AreaRowRange {
    pub(crate) output_y_start: usize,
    pub(crate) output_y_end: usize,
}

impl From<AreaRowRange> for (usize, usize) {
    fn from(row_range: AreaRowRange) -> Self {
        (row_range.output_y_start, row_range.output_y_end)
    }
}

#[doc(hidden)]
pub fn resize_rgba_area_scalar_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        output_rgba.copy_from_slice(source_rgba);
        return Ok(());
    }

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    let x_coverages = prepare_axis_coverages(source_dimensions.width(), output_dimensions.width())?;
    let y_coverages =
        prepare_axis_coverages(source_dimensions.height(), output_dimensions.height())?;

    write_area_rows(
        source_rgba,
        source_width,
        output_row_byte_len,
        &x_coverages,
        &y_coverages,
        AreaRowRange {
            output_y_start: 0,
            output_y_end: output_height,
        },
        output_rgba,
    );

    Ok(())
}

pub(crate) fn write_area_rows<R>(
    source_rgba: &[u8],
    source_width: usize,
    output_row_byte_len: usize,
    x_coverages: &[AxisCoverage],
    y_coverages: &[AxisCoverage],
    row_range: R,
    output_rgba: &mut [u8],
) where
    R: Into<(usize, usize)>,
{
    let (output_y_start, output_y_end) = row_range.into();
    for (output_row, y_coverage) in output_rgba
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_coverages[output_y_start..output_y_end].iter())
    {
        for (output_pixel, x_coverage) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .zip(x_coverages.iter())
        {
            write_area_pixel(
                source_rgba,
                source_width,
                x_coverage,
                y_coverage,
                output_pixel,
            );
        }
    }
}

#[derive(Debug)]
pub(crate) struct AxisCoverage {
    leading_partial: Option<WeightedSourceIndex>,
    full_start: usize,
    full_end: usize,
    trailing_partial: Option<WeightedSourceIndex>,
    inverse_total_weight: f64,
}

#[derive(Debug, Clone, Copy)]
struct WeightedSourceIndex {
    index: usize,
    weight: f64,
}

pub(crate) fn prepare_axis_coverages(
    source_size: u32,
    output_size: u32,
) -> Result<Vec<AxisCoverage>, ProcessingError> {
    let output_len = usize::try_from(output_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "area output axis",
    })?;
    let source_len = usize::try_from(source_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "area source axis",
    })?;
    let scale = f64::from(source_size) / f64::from(output_size);
    let mut coverages = Vec::with_capacity(output_len);

    for output_coordinate in 0..output_len {
        let start = output_coordinate as f64 * scale;
        let end = (output_coordinate + 1) as f64 * scale;
        let first_source = start.floor() as usize;
        let last_source_exclusive = end.ceil() as usize;
        let capped_last_source_exclusive = last_source_exclusive.min(source_len);
        let mut leading_partial = None;
        let mut full_start = capped_last_source_exclusive;
        let mut full_end = capped_last_source_exclusive;
        let mut trailing_partial = None;
        let mut total_weight = 0.0;

        for source_coordinate in first_source..capped_last_source_exclusive {
            let source_start = source_coordinate as f64;
            let source_end = source_start + 1.0;
            let weight = end.min(source_end) - start.max(source_start);

            if weight <= 0.0 {
                continue;
            }

            total_weight += weight;

            if weight == 1.0 {
                if full_start == capped_last_source_exclusive {
                    full_start = source_coordinate;
                }
                full_end = source_coordinate + 1;
                continue;
            }

            let partial = WeightedSourceIndex {
                index: source_coordinate,
                weight,
            };
            if full_start == capped_last_source_exclusive && leading_partial.is_none() {
                leading_partial = Some(partial);
            } else {
                trailing_partial = Some(partial);
            }
        }

        coverages.push(AxisCoverage {
            leading_partial,
            full_start,
            full_end,
            trailing_partial,
            inverse_total_weight: 1.0 / total_weight,
        });
    }

    Ok(coverages)
}

fn write_area_pixel(
    source_rgba: &[u8],
    source_width: usize,
    x_coverage: &AxisCoverage,
    y_coverage: &AxisCoverage,
    output_pixel: &mut [u8],
) {
    let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];

    if let Some(y_sample) = y_coverage.leading_partial {
        accumulate_area_row(
            source_rgba,
            source_width,
            x_coverage,
            y_sample.index,
            y_sample.weight,
            &mut weighted_sums,
        );
    }

    for source_y in y_coverage.full_start..y_coverage.full_end {
        accumulate_area_row(
            source_rgba,
            source_width,
            x_coverage,
            source_y,
            1.0,
            &mut weighted_sums,
        );
    }

    if let Some(y_sample) = y_coverage.trailing_partial {
        accumulate_area_row(
            source_rgba,
            source_width,
            x_coverage,
            y_sample.index,
            y_sample.weight,
            &mut weighted_sums,
        );
    }

    let inverse_total_weight = x_coverage.inverse_total_weight * y_coverage.inverse_total_weight;
    output_pixel[0] = round_channel(weighted_sums[0] * inverse_total_weight);
    output_pixel[1] = round_channel(weighted_sums[1] * inverse_total_weight);
    output_pixel[2] = round_channel(weighted_sums[2] * inverse_total_weight);
    output_pixel[3] = round_channel(weighted_sums[3] * inverse_total_weight);
}

fn accumulate_area_row(
    source_rgba: &[u8],
    source_width: usize,
    x_coverage: &AxisCoverage,
    source_y: usize,
    y_weight: f64,
    weighted_sums: &mut [f64; rgba::RGBA_CHANNEL_COUNT],
) {
    if let Some(x_sample) = x_coverage.leading_partial {
        accumulate_area_sample(
            source_rgba,
            source_width,
            x_sample.index,
            source_y,
            x_sample.weight * y_weight,
            weighted_sums,
        );
    }

    for source_x in x_coverage.full_start..x_coverage.full_end {
        accumulate_area_sample(
            source_rgba,
            source_width,
            source_x,
            source_y,
            y_weight,
            weighted_sums,
        );
    }

    if let Some(x_sample) = x_coverage.trailing_partial {
        accumulate_area_sample(
            source_rgba,
            source_width,
            x_sample.index,
            source_y,
            x_sample.weight * y_weight,
            weighted_sums,
        );
    }
}

fn accumulate_area_sample(
    source_rgba: &[u8],
    source_width: usize,
    source_x: usize,
    source_y: usize,
    sample_weight: f64,
    weighted_sums: &mut [f64; rgba::RGBA_CHANNEL_COUNT],
) {
    let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
    let source_pixel = &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

    weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
    weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
    weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
    weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
}

fn round_channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
#[path = "area_tests.rs"]
mod area_tests;
