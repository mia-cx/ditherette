use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        cpu_tiling::{
            plan_row_bands, process_row_bands_with_plan, RowBand, RowBandPlan, RowBandTiling,
        },
        scalar::nearest::{
            copy_pixel_word, has_wide_source_x_spans, is_exact_integer_downscale,
            prepare_precomputed_offsets, prepare_span_copy_scratch,
            resize_rgba_nearest_scalar_after_fast_paths, PrecomputedOffsetScratch, SpanCopyScratch,
        },
    },
};

struct ExactIntegerDownscalePlan {
    source_row_byte_len: usize,
    output_width: usize,
    y_step: usize,
    first_source_x_byte_offset: usize,
    source_x_byte_step: usize,
}

pub(crate) fn resize_rgba_nearest_with_tiling_after_fast_paths(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: RowBandTiling,
    fallback_to_scalar: bool,
) -> Result<(), ProcessingError> {
    let tiling_plan = if fallback_to_scalar {
        match nearest_tiling_plan(source_dimensions, output_dimensions, tiling)? {
            Some(plan) => plan,
            None => {
                return resize_rgba_nearest_scalar_after_fast_paths(
                    source_rgba,
                    source_dimensions,
                    output_dimensions,
                    output_rgba,
                );
            }
        }
    } else {
        plan_row_bands(
            output_dimensions.width_usize()?,
            output_dimensions.height_usize()?,
            tiling,
        )
    };

    if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        return resize_exact_integer_downscale_tiled_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
            tiling_plan,
        );
    }

    if has_wide_source_x_spans(source_dimensions, output_dimensions)? {
        let mut scratch = SpanCopyScratch::default();
        prepare_span_copy_scratch(source_dimensions, output_dimensions, &mut scratch)?;
        return resize_span_copy_tiled_into(
            source_rgba,
            output_dimensions,
            output_rgba,
            &scratch,
            tiling_plan,
        );
    }

    let mut scratch = PrecomputedOffsetScratch::default();
    prepare_precomputed_offsets(source_dimensions, output_dimensions, &mut scratch)?;
    resize_precomputed_offsets_tiled_into(
        source_rgba,
        output_dimensions,
        output_rgba,
        &scratch,
        tiling_plan,
    )
}

fn resize_exact_integer_downscale_tiled_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let x_step = source_dimensions.width_usize()? / output_width;
    let y_step = source_dimensions.height_usize()? / output_dimensions.height_usize()?;
    let plan = ExactIntegerDownscalePlan {
        source_row_byte_len: source_width * rgba::RGBA_CHANNEL_COUNT,
        output_width,
        y_step,
        first_source_x_byte_offset: x_step / 2 * rgba::RGBA_CHANNEL_COUNT,
        source_x_byte_step: x_step * rgba::RGBA_CHANNEL_COUNT,
    };

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        resize_exact_integer_downscale_rows_into(source_rgba, &plan, band, output_rows);
        Ok(())
    })
}

fn resize_exact_integer_downscale_rows_into(
    source_rgba: &[u8],
    plan: &ExactIntegerDownscalePlan,
    band: RowBand,
    output_rows: &mut [u8],
) {
    let output_row_byte_len = plan.output_width * rgba::RGBA_CHANNEL_COUNT;

    for output_y in band.output_y_start..band.output_y_end {
        let source_y = output_y * plan.y_step + plan.y_step / 2;
        let source_row_offset = source_y * plan.source_row_byte_len;
        let local_output_row_offset = (output_y - band.output_y_start) * output_row_byte_len;
        let mut source_x_offset = plan.first_source_x_byte_offset;
        let mut output_offset = local_output_row_offset;

        for _ in 0..plan.output_width {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rows,
                output_offset,
            );
            source_x_offset += plan.source_x_byte_step;
            output_offset += rgba::RGBA_CHANNEL_COUNT;
        }
    }
}

fn resize_precomputed_offsets_tiled_into(
    source_rgba: &[u8],
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scratch: &PrecomputedOffsetScratch,
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let output_width = output_dimensions.width_usize()?;

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        resize_precomputed_offset_rows_into(
            source_rgba,
            output_width,
            &scratch.source_row_byte_offsets,
            &scratch.source_x_byte_offsets,
            band,
            output_rows,
        );
        Ok(())
    })
}

fn resize_precomputed_offset_rows_into(
    source_rgba: &[u8],
    output_width: usize,
    source_row_byte_offsets: &[usize],
    source_x_byte_offsets: &[usize],
    band: RowBand,
    output_rows: &mut [u8],
) {
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_y, source_row_offset) in source_row_byte_offsets
        .iter()
        .copied()
        .enumerate()
        .take(band.output_y_end)
        .skip(band.output_y_start)
    {
        let local_output_row_offset = (output_y - band.output_y_start) * output_row_byte_len;

        for (output_x, source_x_offset) in source_x_byte_offsets.iter().copied().enumerate() {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rows,
                local_output_row_offset + output_x * rgba::RGBA_CHANNEL_COUNT,
            );
        }
    }
}

fn resize_span_copy_tiled_into(
    source_rgba: &[u8],
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scratch: &SpanCopyScratch,
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let output_width = output_dimensions.width_usize()?;

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        resize_span_copy_rows_into(source_rgba, output_width, scratch, band, output_rows);
        Ok(())
    })
}

fn resize_span_copy_rows_into(
    source_rgba: &[u8],
    output_width: usize,
    scratch: &SpanCopyScratch,
    band: RowBand,
    output_rows: &mut [u8],
) {
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_y, source_row_offset) in scratch
        .source_row_byte_offsets
        .iter()
        .copied()
        .enumerate()
        .take(band.output_y_end)
        .skip(band.output_y_start)
    {
        let local_output_row_offset = (output_y - band.output_y_start) * output_row_byte_len;
        for span in &scratch.source_x_copy_spans {
            let source_start = source_row_offset + span.source_x_byte_offset;
            let output_start = local_output_row_offset + span.output_x_byte_offset;
            output_rows[output_start..output_start + span.byte_len]
                .copy_from_slice(&source_rgba[source_start..source_start + span.byte_len]);
        }
    }
}

pub(crate) fn nearest_tiling_plan(
    _source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    tiling: RowBandTiling,
) -> Result<Option<RowBandPlan>, ProcessingError> {
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let output_pixels =
        output_width
            .checked_mul(output_height)
            .ok_or(ProcessingError::SizeOverflow {
                context: "nearest tiling output pixel count",
            })?;

    if output_pixels < tiling.min_parallel_output_pixels {
        return Ok(None);
    }

    let plan = plan_row_bands(output_width, output_height, tiling);
    if plan.band_count <= 1 {
        return Ok(None);
    }

    Ok(Some(plan))
}
