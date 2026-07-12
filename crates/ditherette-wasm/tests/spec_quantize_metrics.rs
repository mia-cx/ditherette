use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Oklch32, PaletteIndex8, Srgb32},
    spec::quantize::{
        metric::{
            circular_hue3_squared, euclidean3_squared, weighted_rgb_squared, WeightedRgbMetric,
        },
        nearest_color::{
            nearest_ciede2000_index, nearest_circular_hue3_index, nearest_euclidean3_index,
            nearest_weighted_rgb_index, quantize_circular_hue3_into, quantize_euclidean3_into,
            quantize_weighted_rgb_into,
        },
    },
};

#[test]
fn euclidean_metric_uses_squared_cartesian_distance() {
    assert_eq!(euclidean3_squared([1.0, 2.0, 3.0], [4.0, 6.0, 3.0]), 25.0);
}

#[test]
fn nearest_euclidean_ties_use_stable_palette_order() {
    let palette = [[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]];

    assert_eq!(nearest_euclidean3_index([1.0, 0.0, 0.0], &palette), 0);
}

#[test]
fn circular_hue_metric_wraps_around_zero() {
    let low_hue = [0.5, 1.0, 0.05];
    let high_hue = [0.5, 1.0, std::f32::consts::TAU - 0.05];
    let opposite_hue = [0.5, 1.0, std::f32::consts::PI];

    assert!(
        circular_hue3_squared(low_hue, high_hue) < circular_hue3_squared(low_hue, opposite_hue)
    );
}

#[test]
fn nearest_circular_hue_uses_wrapped_hue_distance() {
    let palette = [[0.5, 1.0, 0.2], [0.5, 1.0, std::f32::consts::TAU - 0.02]];

    assert_eq!(nearest_circular_hue3_index([0.5, 1.0, 0.01], &palette), 1);
}

#[test]
fn weighted_rgb_variants_are_srgb_specific_distance_metrics() {
    let red_delta =
        weighted_rgb_squared([0.2, 0.5, 0.5], [0.4, 0.5, 0.5], WeightedRgbMetric::Rec709);
    let green_delta =
        weighted_rgb_squared([0.5, 0.2, 0.5], [0.5, 0.4, 0.5], WeightedRgbMetric::Rec709);

    assert!(green_delta > red_delta);
}

#[test]
fn compuphase_changes_red_and_blue_weights_by_average_red() {
    let dark_red_blue_delta = weighted_rgb_squared(
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.1],
        WeightedRgbMetric::CompuPhase,
    );
    let bright_red_blue_delta = weighted_rgb_squared(
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.1],
        WeightedRgbMetric::CompuPhase,
    );

    assert!(dark_red_blue_delta > bright_red_blue_delta);
}

#[test]
fn ciede2000_nearest_uses_lab_delta_e_2000() {
    let palette = [[50.0, 0.0, -82.7485], [50.0, 2.6772, -79.7751]];

    assert_eq!(nearest_ciede2000_index([50.0, 1.0, -81.0], &palette), 0);
}

#[test]
fn quantize_euclidean_maps_image_pixels_to_palette_indices() {
    let dimensions = ImageDimensions::new(3, 1).unwrap();
    let source = [0.0, 0.0, 0.0, 0.9, 0.1, 0.1, 0.1, 0.9, 0.1];
    let palette = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let mut output = [255; 3];

    quantize_euclidean3_into(
        ImageView::<Srgb32>::packed(&source, dimensions).unwrap(),
        &palette,
        ImageViewMut::<PaletteIndex8>::packed(&mut output, dimensions).unwrap(),
    );

    assert_eq!(output, [0, 1, 2]);
}

#[test]
fn quantize_weighted_rgb_uses_requested_metric() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [0.5, 0.25, 0.5];
    let palette = [[0.5, 0.0, 0.5], [0.5, 0.5, 0.5]];
    let mut output = [255; 1];

    quantize_weighted_rgb_into(
        ImageView::<Srgb32>::packed(&source, dimensions).unwrap(),
        &palette,
        ImageViewMut::<PaletteIndex8>::packed(&mut output, dimensions).unwrap(),
        WeightedRgbMetric::Rec601,
    );

    assert_eq!(output, [0]);
}

#[test]
fn quantize_circular_hue_maps_lch_image_pixels() {
    let dimensions = ImageDimensions::new(1, 1).unwrap();
    let source = [0.5, 1.0, 0.01];
    let palette = [[0.5, 1.0, 0.2], [0.5, 1.0, std::f32::consts::TAU - 0.02]];
    let mut output = [255; 1];

    quantize_circular_hue3_into(
        ImageView::<Oklch32>::packed(&source, dimensions).unwrap(),
        &palette,
        ImageViewMut::<PaletteIndex8>::packed(&mut output, dimensions).unwrap(),
    );

    assert_eq!(output, [1]);
}

#[test]
fn nearest_weighted_rgb_selects_by_weighted_metric() {
    let palette = [[0.2, 0.0, 0.5], [0.2, 0.4, 0.5]];

    assert_eq!(
        nearest_weighted_rgb_index([0.2, 0.3, 0.5], &palette, WeightedRgbMetric::Rec709),
        1
    );
}
