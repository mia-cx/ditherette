use super::{is_exact_integer_downscale, map_output_coordinate, resize_rgba_nearest_into};
#[cfg(feature = "tiling")]
use crate::resize::cpu_tiling::RowBandTiling;
#[cfg(feature = "tiling")]
use crate::resize::tiling::nearest::nearest_tiling_plan;
use crate::{
    image::ImageDimensions,
    resize::{
        nearest::resize_rgba_nearest,
        reference::nearest::{resize_rgba_nearest_reference, write_reference_resize},
    },
};

#[test]
fn maps_coordinates_with_pixel_center_alignment() {
    assert_eq!(map_output_coordinate(0, 4, 2), 1);
    assert_eq!(map_output_coordinate(1, 4, 2), 3);
    assert_eq!(map_output_coordinate(0, 2, 4), 0);
    assert_eq!(map_output_coordinate(3, 2, 4), 1);
}

#[test]
fn detects_exact_integer_downscale() {
    assert!(is_exact_integer_downscale(
        dimensions(4, 4),
        dimensions(2, 2)
    ));
    assert!(!is_exact_integer_downscale(
        dimensions(4, 4),
        dimensions(3, 3)
    ));
    assert!(!is_exact_integer_downscale(
        dimensions(2, 2),
        dimensions(4, 4)
    ));
}

#[cfg(feature = "tiling")]
#[test]
fn nearest_tiling_policy_uses_plan_when_multiple_bands_are_available() {
    let tiling = RowBandTiling::new(1_000_000, 256_000, 256, 5);
    let source_dimensions = dimensions(2600, 4168);
    let near_identity = dimensions(2470, 3959);

    assert!(
        nearest_tiling_plan(source_dimensions, near_identity, tiling)
            .unwrap()
            .is_some()
    );
}

#[test]
fn benchmark_variants_match_baseline_across_resize_shapes() {
    for (source_dimensions, output_dimensions) in [
        (dimensions(6, 4), dimensions(3, 2)),
        (dimensions(7, 5), dimensions(5, 3)),
        (dimensions(3, 2), dimensions(7, 5)),
        (dimensions(4, 5), dimensions(4, 3)),
        (dimensions(4, 3), dimensions(4, 3)),
        (dimensions(1, 1), dimensions(5, 4)),
    ] {
        assert_variants_match_baseline(source_dimensions, output_dimensions);
    }
}

fn assert_variants_match_baseline(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) {
    let source_rgba = patterned_rgba(source_dimensions);
    let baseline =
        resize_rgba_nearest_reference(&source_rgba, source_dimensions, output_dimensions).unwrap();

    assert_eq!(
        resize_rgba_nearest(&source_rgba, source_dimensions, output_dimensions).unwrap(),
        baseline
    );
    let mut output_rgba = vec![0xA5; baseline.len()];
    write_reference_resize(
        &source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .unwrap();
    assert_eq!(output_rgba, baseline);

    let mut output_rgba = vec![0xA5; baseline.len()];
    resize_rgba_nearest_into(
        &source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )
    .unwrap();
    assert_eq!(output_rgba, baseline);
}

fn patterned_rgba(dimensions: ImageDimensions) -> Vec<u8> {
    let pixel_count = dimensions.width() as usize * dimensions.height() as usize;

    (0..pixel_count)
        .flat_map(|index| {
            let value = index as u8;
            [value, value.wrapping_mul(3), value.wrapping_mul(5), 255]
        })
        .collect()
}

fn dimensions(width: u32, height: u32) -> ImageDimensions {
    ImageDimensions::new(width, height).unwrap()
}
