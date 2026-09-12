use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::validate_resize_buffers,
        cpu_tiling::{plan_row_bands, process_row_bands_with_plan, RowBandPlan, RowBandTiling},
        scalar::bilinear::resize_rgba_bilinear_into,
    },
};

const BILINEAR_LARGE_TILING: RowBandTiling = RowBandTiling::new(0, 64_000, 64, 4);
const BILINEAR_NEAR_SOURCE_TILING: RowBandTiling = RowBandTiling::new(0, 64_000, 192, 8);
const BILINEAR_TINY_TWO_BAND_TILING: RowBandTiling = RowBandTiling::new(0, 64_000, 64, 2);
const BILINEAR_SCALAR_OUTPUT_PIXEL_LIMIT: usize = 150_000;
const BILINEAR_TWO_BAND_OUTPUT_PIXEL_LIMIT: usize = 250_000;

// CLOSE(perf): Do not tune old-crate bilinear tiling; this crate is a temporary
// rewrite source and will be removed after current prod parity.

pub(crate) fn resize_rgba_bilinear_with_dynamic_tiling_into(
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

    let Some(plan) = dynamic_bilinear_tiling_plan(source_dimensions, output_dimensions)? else {
        return resize_rgba_bilinear_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        );
    };

    resize_rgba_bilinear_with_plan_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        plan,
    )
}

pub(crate) fn resize_rgba_bilinear_with_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: RowBandTiling,
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        return resize_rgba_bilinear_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        );
    }

    let plan = plan_row_bands(
        output_dimensions.width_usize()?,
        output_dimensions.height_usize()?,
        tiling,
    );
    resize_rgba_bilinear_with_plan_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        plan,
    )
}

pub(crate) fn dynamic_bilinear_tiling_plan(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Option<RowBandPlan>, ProcessingError> {
    if source_dimensions == output_dimensions {
        return Ok(None);
    }

    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let output_pixels =
        output_width
            .checked_mul(output_height)
            .ok_or(ProcessingError::SizeOverflow {
                context: "bilinear tiling output pixel count",
            })?;
    let tiling = if output_pixels < BILINEAR_SCALAR_OUTPUT_PIXEL_LIMIT {
        return Ok(None);
    } else if output_pixels < BILINEAR_TWO_BAND_OUTPUT_PIXEL_LIMIT {
        BILINEAR_TINY_TWO_BAND_TILING
    } else if is_minifying(source_dimensions, output_dimensions) {
        BILINEAR_LARGE_TILING
    } else {
        BILINEAR_NEAR_SOURCE_TILING
    };

    Ok(Some(plan_row_bands(output_width, output_height, tiling)))
}

fn resize_rgba_bilinear_with_plan_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    let y_contributions = prepare_axis_contributions(
        source_dimensions.height(),
        output_dimensions.height(),
        "bilinear tiling output y axis",
    )?;
    let x_contributions = prepare_axis_contributions(
        source_dimensions.width(),
        output_dimensions.width(),
        "bilinear tiling output x axis",
    )?;

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        // CLOSE(perf): Do not tune old-crate scratch allocation; this crate is a
        // temporary rewrite source and will be removed after current prod parity.
        let mut vertical_rgba = vec![0.0; source_row_byte_len];

        for (row_offset, output_row) in output_rows
            .chunks_exact_mut(output_row_byte_len)
            .enumerate()
        {
            let output_y = band.output_y_start + row_offset;
            let y_contribution = &y_contributions[output_y];
            let vertical_row = vertical_rgba.as_mut_slice();
            let (first_weight, remaining_weights) = y_contribution
                .weights
                .split_first()
                .expect("bilinear contributions always have at least one weight");
            let first_source_row_start = y_contribution.first * source_row_byte_len;
            let first_source_row =
                &source_rgba[first_source_row_start..first_source_row_start + source_row_byte_len];

            for (vertical_pixel, source_pixel) in vertical_row
                .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                .zip(first_source_row.chunks_exact(rgba::RGBA_CHANNEL_COUNT))
            {
                vertical_pixel[0] = f32::from(source_pixel[0]) * first_weight;
                vertical_pixel[1] = f32::from(source_pixel[1]) * first_weight;
                vertical_pixel[2] = f32::from(source_pixel[2]) * first_weight;
                vertical_pixel[3] = f32::from(source_pixel[3]) * first_weight;
            }

            for (weight_offset, weight) in remaining_weights.iter().enumerate() {
                let source_y = y_contribution.first + weight_offset + 1;
                let source_row_start = source_y * source_row_byte_len;
                let source_row =
                    &source_rgba[source_row_start..source_row_start + source_row_byte_len];

                for (vertical_pixel, source_pixel) in vertical_row
                    .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                    .zip(source_row.chunks_exact(rgba::RGBA_CHANNEL_COUNT))
                {
                    vertical_pixel[0] += f32::from(source_pixel[0]) * weight;
                    vertical_pixel[1] += f32::from(source_pixel[1]) * weight;
                    vertical_pixel[2] += f32::from(source_pixel[2]) * weight;
                    vertical_pixel[3] += f32::from(source_pixel[3]) * weight;
                }
            }

            for (output_pixel, x_contribution) in output_row
                .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                .zip(&x_contributions)
            {
                let mut red = 0.0;
                let mut green = 0.0;
                let mut blue = 0.0;
                let mut alpha = 0.0;

                for (weight_index, weight) in x_contribution.weights.iter().enumerate() {
                    let source_x = x_contribution.first + weight_index;
                    let vertical_offset = source_x * rgba::RGBA_CHANNEL_COUNT;

                    red += vertical_row[vertical_offset] * weight;
                    green += vertical_row[vertical_offset + 1] * weight;
                    blue += vertical_row[vertical_offset + 2] * weight;
                    alpha += vertical_row[vertical_offset + 3] * weight;
                }

                output_pixel[0] = round_u8(red);
                output_pixel[1] = round_u8(green);
                output_pixel[2] = round_u8(blue);
                output_pixel[3] = round_u8(alpha);
            }
        }

        Ok(())
    })
}

fn prepare_axis_contributions(
    source_size: u32,
    output_size: u32,
    context: &'static str,
) -> Result<Vec<AxisContribution>, ProcessingError> {
    let source_len =
        usize::try_from(source_size).map_err(|_| ProcessingError::SizeOverflow { context })?;
    let output_len =
        usize::try_from(output_size).map_err(|_| ProcessingError::SizeOverflow { context })?;
    let ratio = source_size as f32 / output_size as f32;
    let scale = ratio.max(1.0);
    let support = scale;
    let mut contributions = Vec::with_capacity(output_len);

    for output_coordinate in 0..output_len {
        let input = (output_coordinate as f32 + 0.5) * ratio;
        let left = clamp_i64(
            (input - support).floor() as i64,
            0,
            i64::from(source_size) - 1,
        ) as usize;
        let right = clamp_i64(
            (input + support).ceil() as i64,
            i64::try_from(left + 1).map_err(|_| ProcessingError::SizeOverflow { context })?,
            i64::from(source_size),
        ) as usize;
        let center = input - 0.5;
        let mut first = left;
        let mut weights = Vec::with_capacity(right - left);
        let mut sum = 0.0;

        for source_coordinate in left..right {
            let weight = triangle_weight((source_coordinate as f32 - center) / scale);
            if weight == 0.0 && weights.is_empty() {
                first += 1;
                continue;
            }

            weights.push(weight);
            sum += weight;
        }

        while weights.last() == Some(&0.0) {
            weights.pop();
        }

        for weight in &mut weights {
            *weight /= sum;
        }

        contributions.push(AxisContribution { first, weights });
    }

    debug_assert_eq!(contributions.len(), output_len);
    debug_assert!(contributions
        .iter()
        .all(|contribution| contribution.first < source_len));

    Ok(contributions)
}

fn is_minifying(source_dimensions: ImageDimensions, output_dimensions: ImageDimensions) -> bool {
    output_dimensions.width() < source_dimensions.width()
        || output_dimensions.height() < source_dimensions.height()
}

fn triangle_weight(distance: f32) -> f32 {
    if distance.abs() < 1.0 {
        1.0 - distance.abs()
    } else {
        0.0
    }
}

fn round_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

#[derive(Debug, Clone)]
struct AxisContribution {
    first: usize,
    weights: Vec<f32>,
}
