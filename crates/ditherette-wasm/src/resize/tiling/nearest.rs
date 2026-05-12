use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        cpu_tiling::{plan_row_bands, RowBandPlan, RowBandTiling},
        scalar::nearest::{
            has_wide_source_x_spans, is_exact_integer_downscale, prepare_precomputed_offsets,
            prepare_span_copy_scratch, resize_exact_integer_downscale_tiled_into,
            resize_precomputed_offsets_tiled_into, resize_rgba_nearest_scalar_after_fast_paths,
            resize_span_copy_tiled_into, PrecomputedOffsetScratch, SpanCopyScratch,
        },
    },
};

pub(crate) fn resize_rgba_nearest_with_tiling_after_fast_paths(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: RowBandTiling,
    fallback_to_scalar: bool,
) -> Result<(), ProcessingError> {
    resize_rgba_nearest_with_tiling_after_fast_paths_inner(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        tiling,
        fallback_to_scalar,
    )
}

fn resize_rgba_nearest_with_tiling_after_fast_paths_inner(
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
