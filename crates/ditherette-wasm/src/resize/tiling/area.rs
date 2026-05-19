use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::validate_resize_buffers,
        cpu_tiling::{
            plan_row_bands, process_row_bands_with_plan, RowBand, RowBandPlan, RowBandTiling,
        },
    },
};

pub(crate) const AREA_ROW_BAND_TILING: RowBandTiling = RowBandTiling::new(750_000, 64_000, 192, 8);

const AREA_FOUR_BAND_TILING: RowBandTiling = RowBandTiling::new(0, 64_000, 64, 4);
const AREA_SMALL_FRACTIONAL_TILING: RowBandTiling = RowBandTiling::new(0, 64_000, 192, 8);
const AREA_TINY_OUTPUT_PIXEL_LIMIT: usize = 200_000;
const AREA_SMALL_FRACTIONAL_OUTPUT_PIXEL_LIMIT: usize = 500_000;

impl From<RowBand> for (usize, usize) {
    fn from(row_band: RowBand) -> Self {
        (row_band.output_y_start, row_band.output_y_end)
    }
}

pub(crate) fn resize_rgba_area_with_dynamic_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    let Some(plan) = dynamic_area_tiling_plan(source_dimensions, output_dimensions)? else {
        return crate::resize::scalar::area::resize_rgba_area_scalar_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        );
    };

    resize_rgba_area_with_plan(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        plan,
    )
}

pub(crate) fn resize_rgba_area_with_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: RowBandTiling,
) -> Result<(), ProcessingError> {
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    resize_rgba_area_with_plan(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        plan_row_bands(output_width, output_height, tiling),
    )
}

fn resize_rgba_area_with_plan(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    plan: RowBandPlan,
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

    let output_height = output_dimensions.height_usize()?;
    let kernel = AreaKernel::new(source_rgba, source_dimensions, output_dimensions)?;

    if plan.band_count <= 1 {
        kernel.write_rows((0, output_height), output_rgba);
        return Ok(());
    }

    process_area_rows_with_plan(kernel, output_rgba, plan)
}

fn process_area_rows_with_plan(
    kernel: AreaKernel<'_>,
    output_rgba: &mut [u8],
    plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    process_row_bands_with_plan(output_rgba, plan, |band, output_rows| {
        kernel.write_rows(band, output_rows);
        Ok(())
    })
}

pub(crate) fn dynamic_area_tiling_plan(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Option<RowBandPlan>, ProcessingError> {
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let output_pixels =
        output_width
            .checked_mul(output_height)
            .ok_or(ProcessingError::SizeOverflow {
                context: "area tiling output pixel count",
            })?;

    if output_pixels < AREA_TINY_OUTPUT_PIXEL_LIMIT
        || (output_dimensions.width() >= source_dimensions.width()
            && output_dimensions.height() >= source_dimensions.height())
    {
        return Ok(None);
    }

    // Chosen from the dense area tiling sweep over the Celeste fixture and
    // checked against Criterion after area_2 became canonical scalar area. The
    // dimensions matter more than nominal scale labels: exact strong minifiers
    // stay scalar, scalar keeps the faster optimized upscale path, exact 2x
    // downscales prefer four row bands, near-source minification prefers the
    // default five-worker shape, and small fractional outputs need enough row
    // height to amortize scheduling.
    let tiling = if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        let x_step = source_dimensions.width() / output_dimensions.width();
        let y_step = source_dimensions.height() / output_dimensions.height();
        if x_step >= 4 || y_step >= 4 {
            return Ok(None);
        }
        AREA_FOUR_BAND_TILING
    } else if is_near_source_size(source_dimensions, output_dimensions) {
        AREA_ROW_BAND_TILING
    } else if output_pixels < AREA_SMALL_FRACTIONAL_OUTPUT_PIXEL_LIMIT {
        AREA_SMALL_FRACTIONAL_TILING
    } else if output_dimensions.width() < source_dimensions.width()
        || output_dimensions.height() < source_dimensions.height()
    {
        AREA_FOUR_BAND_TILING
    } else {
        AREA_ROW_BAND_TILING
    };

    let plan = plan_row_bands(output_width, output_height, tiling);
    if plan.band_count <= 1 {
        return Ok(None);
    }

    Ok(Some(plan))
}

fn is_near_source_size(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    u64::from(output_dimensions.width()) * 10 > u64::from(source_dimensions.width()) * 9
        || u64::from(output_dimensions.height()) * 10 > u64::from(source_dimensions.height()) * 9
}

fn is_exact_integer_downscale(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    let source_width = source_dimensions.width();
    let source_height = source_dimensions.height();
    let output_width = output_dimensions.width();
    let output_height = output_dimensions.height();

    source_width >= output_width
        && source_height >= output_height
        && (source_width > output_width || source_height > output_height)
        && source_width.is_multiple_of(output_width)
        && source_height.is_multiple_of(output_height)
}

#[derive(Debug, Clone, Copy)]
struct AreaKernel<'a> {
    source_rgba: &'a [u8],
    source_width: usize,
    source_width_u32: u32,
    source_height: u32,
    output_row_byte_len: usize,
    x_scale: f64,
    y_scale: f64,
}

impl<'a> AreaKernel<'a> {
    fn new(
        source_rgba: &'a [u8],
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
    ) -> Result<Self, ProcessingError> {
        let output_width = output_dimensions.width_usize()?;

        Ok(Self {
            source_rgba,
            source_width: source_dimensions.width_usize()?,
            source_width_u32: source_dimensions.width(),
            source_height: source_dimensions.height(),
            output_row_byte_len: output_width * rgba::RGBA_CHANNEL_COUNT,
            x_scale: f64::from(source_dimensions.width()) / f64::from(output_dimensions.width()),
            y_scale: f64::from(source_dimensions.height()) / f64::from(output_dimensions.height()),
        })
    }

    fn write_rows<R>(self, row_range: R, output_rgba: &mut [u8])
    where
        R: Into<(usize, usize)>,
    {
        let (output_y_start, output_y_end) = row_range.into();

        for (output_y, output_row) in (output_y_start..output_y_end)
            .zip(output_rgba.chunks_exact_mut(self.output_row_byte_len))
        {
            self.write_row(output_y, output_row);
        }
    }

    fn write_row(self, output_y: usize, output_row: &mut [u8]) {
        let y_range = SourceRange::for_output_pixel(output_y, self.y_scale, self.source_height);

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .enumerate()
        {
            self.write_pixel(output_x, y_range, output_pixel);
        }
    }

    fn write_pixel(self, output_x: usize, y_range: SourceRange, output_pixel: &mut [u8]) {
        let x_range = SourceRange::for_output_pixel(output_x, self.x_scale, self.source_width_u32);
        let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
        let mut total_weight = 0.0;

        for source_y in y_range.first..y_range.last_exclusive {
            let y_weight = y_range.overlap_with(source_y);

            for source_x in x_range.first..x_range.last_exclusive {
                let x_weight = x_range.overlap_with(source_x);
                let sample_weight = x_weight * y_weight;
                let source_offset = rgba::pixel_byte_offset(self.source_width, source_x, source_y);
                let source_pixel =
                    &self.source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

                weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
                weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
                weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
                weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
                total_weight += sample_weight;
            }
        }

        output_pixel[0] = round_channel(weighted_sums[0] / total_weight);
        output_pixel[1] = round_channel(weighted_sums[1] / total_weight);
        output_pixel[2] = round_channel(weighted_sums[2] / total_weight);
        output_pixel[3] = round_channel(weighted_sums[3] / total_weight);
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceRange {
    start: f64,
    end: f64,
    first: usize,
    last_exclusive: usize,
}

impl SourceRange {
    fn for_output_pixel(output_coordinate: usize, scale: f64, source_size: u32) -> Self {
        let start = output_coordinate as f64 * scale;
        let end = (output_coordinate + 1) as f64 * scale;
        let first = start.floor() as usize;
        let last_exclusive = (end.ceil() as usize).min(source_size as usize);

        Self {
            start,
            end,
            first,
            last_exclusive,
        }
    }

    fn overlap_with(self, source_coordinate: usize) -> f64 {
        let source_start = source_coordinate as f64;
        let source_end = source_start + 1.0;

        (self.end.min(source_end) - self.start.max(source_start)).max(0.0)
    }
}

fn round_channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }

    #[test]
    fn dynamic_plan_keeps_upscale_scalar_and_uses_default_shape_for_near_source() {
        assert!(
            dynamic_area_tiling_plan(dimensions(2600, 4168), dimensions(5200, 8336))
                .unwrap()
                .is_none()
        );

        let plan = dynamic_area_tiling_plan(dimensions(2600, 4168), dimensions(2470, 3959))
            .unwrap()
            .unwrap();
        assert_eq!(plan.min_rows_per_band, 192);
        assert_eq!(plan.max_workers, 8);
    }

    #[test]
    fn dynamic_plan_uses_four_band_shape_for_common_downscales() {
        let plan = dynamic_area_tiling_plan(dimensions(2600, 4168), dimensions(1300, 2084))
            .unwrap()
            .unwrap();
        assert_eq!(plan.min_pixels_per_band, 64_000);
        assert_eq!(plan.min_rows_per_band, 64);
        assert_eq!(plan.max_workers, 4);

        assert!(
            dynamic_area_tiling_plan(dimensions(2600, 4168), dimensions(650, 1042))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn dynamic_plan_keeps_tiny_exact_downscales_scalar() {
        assert!(
            dynamic_area_tiling_plan(dimensions(2600, 4168), dimensions(325, 521))
                .unwrap()
                .is_none()
        );
    }
}
