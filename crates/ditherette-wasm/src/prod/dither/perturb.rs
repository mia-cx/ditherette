//! Literal field-to-RGBA8 row composition with caller-owned output.

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::{
        color::{packed::rgb8_to_coordinates, reconstruct},
        contract::request::{Placement, WorkingSpace},
        tiling::RowBand,
    },
};

use super::placement::{coordinate_domain, placement_mask_at};

const FIELD_SCALE: f64 = 0.25;

/// Shared field composition. The callback returns one threshold in [-0.5,0.5] per global pixel.
/// It runs once per written pixel even when alpha, strength, or placement would suppress an effect.
pub fn perturb_by_field_rows_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    space: WorkingSpace,
    strength: f32,
    placement: Placement,
    rows: RowBand,
    field: impl Fn(u32, u32, u64) -> f32,
) {
    let dimensions = source.dimensions();
    assert_eq!(dimensions, output.dimensions());
    assert!(rows.y_end() <= dimensions.height());
    let ranges = coordinate_domain(space).ranges().map(f64::from);
    for y in rows.y_start()..rows.y_end() {
        let source_row = source.row(y).expect("source row is in bounds");
        let output_row = output.row_mut(y).expect("output row is in bounds");
        for x in 0..dimensions.width() {
            let global_index = u64::from(y) * u64::from(dimensions.width()) + u64::from(x);
            let threshold = field(x, y, global_index);
            let mask = placement_mask_at(source, x, y, space, placement);
            let amount = f64::from(threshold) * f64::from(strength) * f64::from(mask) * FIELD_SCALE;
            let offset = x as usize * 4;
            let pixel = &source_row[offset..offset + 4];
            let rgb = [pixel[0], pixel[1], pixel[2]];
            let result = if amount == 0.0 {
                rgb
            } else {
                let coordinates = rgb8_to_coordinates(rgb, space);
                let perturbed = std::array::from_fn(|axis| {
                    f64::from(coordinates[axis]) + amount * ranges[axis]
                });
                reconstruct::coordinates_to_rgb8(perturbed, space)
            };
            output_row[offset..offset + 3].copy_from_slice(&result);
            output_row[offset + 3] = pixel[3];
        }
    }
}
