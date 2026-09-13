//! Palette-free fields with an explicit RGBA8 reconstruction boundary.

use crate::{
    image::{ImageBuf, ImageLayoutError, ImageView, ImageViewMut, Rgba8},
    spec::{
        color::{cielab, cielch, linear, oklab, oklch, reconstruct, srgb, ycbcr},
        contract::request::{BayerSize, Field, PerturbPolicy, Placement, WorkingSpace},
        tiling::contract::RowBand,
    },
};

use super::{
    blue_noise, ordered,
    placement::{coordinate_domain, placement_mask_at},
    random_noise,
};

const FIELD_SCALE: f64 = 0.25;

/// Allocates durable RGBA8 output from a request-validated source and policy.
/// Alpha bytes pass through unchanged; no palette or indexed alpha policy is consulted.
pub fn perturb(
    source: ImageView<'_, Rgba8>,
    policy: PerturbPolicy,
) -> Result<ImageBuf<Rgba8>, ImageLayoutError> {
    let mut output = ImageBuf::new_packed(source.dimensions())?;
    perturb_into(source, output.as_view_mut(), policy);
    Ok(output)
}

/// Materializes clipped, rounded RGBA8 before invoking the supplied quantization composition.
/// The callback receives bytes only, so it cannot bypass this reconstruction boundary.
pub fn quantize_after_perturb<T>(
    source: ImageView<'_, Rgba8>,
    policy: PerturbPolicy,
    quantize: impl FnOnce(ImageView<'_, Rgba8>) -> T,
) -> Result<T, ImageLayoutError> {
    let intermediate = perturb(source, policy)?;
    Ok(quantize(intermediate.as_view()))
}

/// Writes all source pixels into an equally sized RGBA8 view, leaving output padding unchanged.
pub fn perturb_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    policy: PerturbPolicy,
) {
    let rows = RowBand::new(0, source.dimensions().height()).expect("image height is nonzero");
    perturb_rows_into(source, output, policy, rows);
}

/// Writes only the requested output rows using full-image coordinates and the full source neighborhood.
pub fn perturb_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    policy: PerturbPolicy,
    rows: RowBand,
) {
    perturb_by_field_rows_into(
        source,
        output,
        policy.space,
        policy.strength,
        policy.placement,
        rows,
        |x, y, index| field_at(policy.field, x, y, index),
    );
}

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

fn field_at(field: Field, x: u32, y: u32, index: u64) -> f32 {
    match field {
        Field::Bayer { size } => {
            let size = match size {
                BayerSize::Two => ordered::BayerSize::Two,
                BayerSize::Four => ordered::BayerSize::Four,
                BayerSize::Eight => ordered::BayerSize::Eight,
                BayerSize::Sixteen => ordered::BayerSize::Sixteen,
            };
            ordered::bayer_noise_at(x, y, size)
        }
        Field::Random { seed } => random_noise::random_noise_at(seed, index),
        Field::BlueNoise => blue_noise::blue_noise_at(x, y),
    }
}

fn rgb8_to_coordinates(rgb: [u8; 3], space: WorkingSpace) -> [f32; 3] {
    match space {
        WorkingSpace::Srgb => srgb::rgb8_to_srgb(rgb),
        WorkingSpace::LinearRgb => linear::rgb8_to_linear_rgb(rgb),
        WorkingSpace::Oklab => oklab::rgb8_to_oklab(rgb),
        WorkingSpace::Oklch => oklch::rgb8_to_oklch(rgb),
        WorkingSpace::Cielab => cielab::rgb8_to_cielab(rgb),
        WorkingSpace::Cielch => cielch::rgb8_to_cielch(rgb),
        WorkingSpace::Ycbcr => ycbcr::rgb8_to_ycbcr(rgb),
    }
}
