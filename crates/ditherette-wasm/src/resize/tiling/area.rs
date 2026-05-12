use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::validate_resize_buffers,
        cpu_tiling::{
            plan_row_bands, process_row_bands_with_plan, RowBand, RowBandPlan, RowBandTiling,
        },
        scalar::area::{prepare_axis_coverages, write_area_rows, AreaRowRange, AxisCoverage},
    },
};

pub(crate) const AREA_ROW_BAND_TILING: RowBandTiling = RowBandTiling::new(0, 64_000, 192, 4);

impl From<RowBand> for (usize, usize) {
    fn from(row_band: RowBand) -> Self {
        (row_band.output_y_start, row_band.output_y_end)
    }
}

pub(crate) fn resize_rgba_area_with_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: RowBandTiling,
) -> Result<(), ProcessingError> {
    resize_rgba_area_with_tiling(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        tiling,
    )
}

fn resize_rgba_area_with_tiling(
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
    let plan = plan_row_bands(output_width, output_height, tiling);

    if plan.band_count <= 1 {
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
        return Ok(());
    }

    process_area_rows_with_plan(
        source_rgba,
        source_width,
        output_row_byte_len,
        &x_coverages,
        &y_coverages,
        output_rgba,
        plan,
    )
}

fn process_area_rows_with_plan(
    source_rgba: &[u8],
    source_width: usize,
    output_row_byte_len: usize,
    x_coverages: &[AxisCoverage],
    y_coverages: &[AxisCoverage],
    output_rgba: &mut [u8],
    plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    process_row_bands_with_plan(output_rgba, plan, |band, output_rows| {
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
    })
}
