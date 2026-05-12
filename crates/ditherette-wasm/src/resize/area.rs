use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

#[cfg(feature = "tiling")]
use crate::resize::cpu_tiling::{process_row_bands, RowBand, DEFAULT_ROW_BAND_TILING};

/// Resizes RGBA with exact pixel-area averaging.
///
/// This is the practical "box" downsampling filter: every output pixel covers a
/// rectangle in source-pixel space, and each covered source pixel contributes by
/// its overlap area. It is a strong reference implementation for antialiased
/// minification, though it can look blocky for upscaling.
pub fn resize_rgba_area(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_area_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with exact area averaging.
pub fn resize_rgba_area_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Reuse area scratch state across calls like nearest does. The
    // coverage vectors are shape-specific and currently allocate on every resize.
    resize_rgba_area_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

/// Straightforward reference implementation for exact pixel-area averaging.
#[doc(hidden)]
pub fn resize_rgba_area_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_area_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the area reference implementation.
#[doc(hidden)]
pub fn resize_rgba_area_reference_into(
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
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    let x_coverages = prepare_axis_coverages(source_dimensions.width(), output_dimensions.width())?;
    let y_coverages =
        prepare_axis_coverages(source_dimensions.height(), output_dimensions.height())?;

    process_area_rows(
        source_rgba,
        source_width,
        output_width,
        output_dimensions.height_usize()?,
        output_row_byte_len,
        &x_coverages,
        &y_coverages,
        output_rgba,
    )
}

#[cfg(not(feature = "tiling"))]
#[derive(Debug, Clone, Copy)]
struct AreaRowRange {
    output_y_start: usize,
    output_y_end: usize,
}

#[cfg(not(feature = "tiling"))]
impl From<AreaRowRange> for (usize, usize) {
    fn from(row_range: AreaRowRange) -> Self {
        (row_range.output_y_start, row_range.output_y_end)
    }
}

#[cfg(feature = "tiling")]
impl From<RowBand> for (usize, usize) {
    fn from(row_band: RowBand) -> Self {
        (row_band.output_y_start, row_band.output_y_end)
    }
}

#[cfg(feature = "tiling")]
fn process_area_rows(
    source_rgba: &[u8],
    source_width: usize,
    output_width: usize,
    output_height: usize,
    output_row_byte_len: usize,
    x_coverages: &[AxisCoverage],
    y_coverages: &[AxisCoverage],
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    process_row_bands(
        output_rgba,
        output_width,
        output_height,
        DEFAULT_ROW_BAND_TILING,
        |band, output_rows| {
            write_area_rows(
                source_rgba,
                source_width,
                output_row_byte_len,
                x_coverages,
                y_coverages,
                band,
                output_rows,
            );
            Ok(())
        },
    )
}

#[cfg(not(feature = "tiling"))]
fn process_area_rows(
    source_rgba: &[u8],
    source_width: usize,
    _output_width: usize,
    output_height: usize,
    output_row_byte_len: usize,
    x_coverages: &[AxisCoverage],
    y_coverages: &[AxisCoverage],
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    write_area_rows(
        source_rgba,
        source_width,
        output_row_byte_len,
        x_coverages,
        y_coverages,
        AreaRowRange {
            output_y_start: 0,
            output_y_end: output_height,
        },
        output_rgba,
    );
    Ok(())
}

fn write_area_rows<R>(
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
struct AxisCoverage {
    samples: Vec<WeightedSourceIndex>,
    total_weight: f64,
}

#[derive(Debug)]
struct WeightedSourceIndex {
    index: usize,
    weight: f64,
}

fn prepare_axis_coverages(
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
        let mut samples = Vec::with_capacity(capped_last_source_exclusive - first_source);
        let mut total_weight = 0.0;

        for source_coordinate in first_source..capped_last_source_exclusive {
            let source_start = source_coordinate as f64;
            let source_end = source_start + 1.0;
            let weight = end.min(source_end) - start.max(source_start);

            if weight <= 0.0 {
                continue;
            }

            samples.push(WeightedSourceIndex {
                index: source_coordinate,
                weight,
            });
            total_weight += weight;
        }

        coverages.push(AxisCoverage {
            samples,
            total_weight,
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

    for y_sample in &y_coverage.samples {
        for x_sample in &x_coverage.samples {
            let source_offset =
                rgba::pixel_byte_offset(source_width, x_sample.index, y_sample.index);
            let sample_weight = x_sample.weight * y_sample.weight;
            let source_pixel =
                &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

            weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
            weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
            weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
            weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
        }
    }

    let total_weight = x_coverage.total_weight * y_coverage.total_weight;
    output_pixel[0] = round_channel(weighted_sums[0] / total_weight);
    output_pixel[1] = round_channel(weighted_sums[1] / total_weight);
    output_pixel[2] = round_channel(weighted_sums[2] / total_weight);
    output_pixel[3] = round_channel(weighted_sums[3] / total_weight);
}

fn round_channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba_area, resize_rgba_area_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn four_by_four_to_two_by_two_averages_covered_pixels() {
        let source_rgba: Vec<u8> = (0..16)
            .flat_map(|value| [value, value, value, 255])
            .collect();

        let output_rgba =
            resize_rgba_area(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap();
        let reference_rgba =
            resize_rgba_area_reference(&source_rgba, dimensions(4, 4), dimensions(2, 2)).unwrap();

        let expected_rgba = [
            [3, 3, 3, 255],
            [5, 5, 5, 255],
            [11, 11, 11, 255],
            [13, 13, 13, 255],
        ]
        .concat();

        assert_eq!(output_rgba, expected_rgba);
        assert_eq!(reference_rgba, expected_rgba);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
