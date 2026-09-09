//! Field-to-RGBA8 row composition with caller-owned output and one shared color converter.

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        color::{
            packed::{Converter, PackedSpace},
            reconstruct,
        },
        contract::{
            failure::Failure,
            request::{Placement, WorkingSpace},
        },
        resize::common::allocation::CapacityBudget,
        tiling::{bands_for_output_height, RowBand, RowBandBuffers, WorkerBudget},
    },
};

use super::placement::{coordinate_domain, placement_mask_with_converter};

const FIELD_SCALE: f64 = 0.25;

/// Each active field worker can own one temporary converter, including its byte tables.
/// The enclosing call charges this in addition to row metadata, source, and output.
pub const fn band_working_capacity_bytes(active_workers: u32) -> u64 {
    std::mem::size_of::<crate::prod::color::packed::Converter>() as u64 * active_workers as u64
}

/// Counts assignment ownership and each concurrently live converter before allocation.
pub fn required_band_capacity_bytes(
    dimensions: ImageDimensions,
    band_height: u32,
    workers: WorkerBudget,
    requested_workers: u32,
) -> Result<u64, Failure> {
    let metadata = RowBandBuffers::<()>::required_bytes(
        dimensions,
        band_height,
        workers,
        requested_workers,
        &|_| Ok(0),
    )?;
    let bands = bands_for_output_height(dimensions, band_height).expect("validated band height");
    let active = workers.active_workers(requested_workers, bands.len() as u32);
    Ok(metadata + band_working_capacity_bytes(active))
}

/// Reserves row metadata only after the complete field working set fits.
/// Temporary converters stay on worker stacks; callers keep their charge until execution joins.
pub fn try_band_buffers(
    dimensions: ImageDimensions,
    band_height: u32,
    workers: WorkerBudget,
    requested_workers: u32,
    limit: u64,
) -> Result<RowBandBuffers<()>, Failure> {
    let required =
        required_band_capacity_bytes(dimensions, band_height, workers, requested_workers)?;
    CapacityBudget::new(limit).check_additional(required)?;
    let bands = bands_for_output_height(dimensions, band_height).expect("validated band height");
    let active = workers.active_workers(requested_workers, bands.len() as u32);
    RowBandBuffers::try_new(
        dimensions,
        band_height,
        workers,
        requested_workers,
        limit - band_working_capacity_bytes(active),
        &|_| Ok(0),
    )
}

/// Executes preflighted disjoint bands while preserving the full immutable source.
/// Field evaluation stays on workers; completed-row callbacks run only after joining.
pub fn perturb_by_field_bands_into(
    source: ImageView<'_, Rgba8>,
    output: &mut [u8],
    space: WorkingSpace,
    strength: f32,
    placement: Placement,
    work: &mut RowBandBuffers<()>,
    field: impl Fn(u32, u32, u64) -> f32 + Sync,
    progress: &mut impl FnMut(u64) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let width = source.dimensions().width();
    work.execute(
        output,
        width as usize * 4,
        &|band, output, _| {
            let dimensions = ImageDimensions::new(width, band.height()).expect("validated band");
            perturb_by_field_band_into(
                source,
                ImageViewMut::packed(output, dimensions).expect("disjoint packed output"),
                space,
                strength,
                placement,
                band,
                &field,
            );
            Ok(u64::from(band.height()))
        },
        progress,
    )
}

/// Shared field composition. The callback returns one threshold in [-0.5,0.5] per global pixel.
/// It runs once per written pixel even when alpha, strength, or placement would suppress an effect.
pub fn perturb_by_field_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    space: WorkingSpace,
    strength: f32,
    placement: Placement,
    rows: RowBand,
    field: impl Fn(u32, u32, u64) -> f32,
) {
    perturb_by_field_with_progress(
        source,
        output,
        space,
        strength,
        placement,
        rows,
        field,
        |_| Ok(()),
    )
    .expect("disabled progress cannot fail");
}

/// Keeps global field indices and full-source adaptive neighbors while reporting completed rows.
pub(crate) fn perturb_by_field_with_progress(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    space: WorkingSpace,
    strength: f32,
    placement: Placement,
    rows: RowBand,
    field: impl Fn(u32, u32, u64) -> f32,
    progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    assert_eq!(source.dimensions(), output.dimensions());
    perturb_rows(
        source, output, space, strength, placement, rows, 0, field, progress,
    )
}

/// Writes only a band-local output view while retaining full-source adaptive reads.
/// Output row zero corresponds to `rows.y_start()`; field coordinates remain absolute.
pub fn perturb_by_field_band_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    space: WorkingSpace,
    strength: f32,
    placement: Placement,
    rows: RowBand,
    field: impl Fn(u32, u32, u64) -> f32,
) {
    assert_eq!(source.dimensions().width(), output.dimensions().width());
    assert_eq!(rows.height(), output.dimensions().height());
    perturb_rows(
        source,
        output,
        space,
        strength,
        placement,
        rows,
        rows.y_start(),
        field,
        |_| Ok(()),
    )
    .expect("disabled progress cannot fail");
}

fn perturb_rows(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    space: WorkingSpace,
    strength: f32,
    placement: Placement,
    rows: RowBand,
    output_y_start: u32,
    field: impl Fn(u32, u32, u64) -> f32,
    mut progress: impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    let dimensions = source.dimensions();
    assert!(rows.y_end() <= dimensions.height());
    let ranges = coordinate_domain(space).ranges().map(f64::from);
    let converter = Converter::new(PackedSpace::from_working(space));
    for y in rows.y_start()..rows.y_end() {
        let source_row = source.row(y).expect("source row is in bounds");
        let output_row = output
            .row_mut(y - output_y_start)
            .expect("output row is in bounds");
        for x in 0..dimensions.width() {
            let global_index = u64::from(y) * u64::from(dimensions.width()) + u64::from(x);
            let threshold = field(x, y, global_index);
            let mask = placement_mask_with_converter(source, x, y, space, placement, &converter);
            let amount = f64::from(threshold) * f64::from(strength) * f64::from(mask) * FIELD_SCALE;
            let offset = x as usize * 4;
            let pixel = &source_row[offset..offset + 4];
            let rgb = [pixel[0], pixel[1], pixel[2]];
            let result = if amount == 0.0 {
                rgb
            } else {
                let coordinates = converter.coordinates(rgb);
                let perturbed = std::array::from_fn(|axis| {
                    f64::from(coordinates[axis]) + amount * ranges[axis]
                });
                reconstruct::coordinates_to_rgb8(perturbed, space)
            };
            output_row[offset..offset + 3].copy_from_slice(&result);
            output_row[offset + 3] = pixel[3];
        }
        progress(y + 1)?;
    }
    Ok(())
}
