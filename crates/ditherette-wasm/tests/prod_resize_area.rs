use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::scalar::area::resize_area_rgba8_into as resize_prod_area_into,
    spec::resize::scalar::area::resize_area_into as resize_spec_area_into,
};

const MAX_COLOR_DISTANCE: f64 = 2.0;
const MAX_MEAN_COLOR_DISTANCE: f64 = 2.0;
const MAX_RMS_COLOR_DISTANCE: f64 = 2.0;

#[test]
fn prod_area_matches_spec_for_exact_integer_rgba8_resizes() {
    let source_dimensions = ImageDimensions::new(8, 8).unwrap();
    let source = patterned_rgba_source(source_dimensions);

    for output_dimensions in [
        ImageDimensions::new(4, 4).unwrap(),
        ImageDimensions::new(2, 2).unwrap(),
        ImageDimensions::new(1, 1).unwrap(),
        ImageDimensions::new(16, 16).unwrap(),
    ] {
        let (spec_output, prod_output) =
            resize_outputs(&source, source_dimensions, output_dimensions);
        assert_eq!(prod_output, spec_output, "output {output_dimensions:?}");
    }
}

#[test]
fn prod_area_stays_within_bounded_oracle_for_fractional_rgba8_resizes() {
    let source_dimensions = ImageDimensions::new(5, 4).unwrap();
    let source = patterned_rgba_source(source_dimensions);

    for output_dimensions in [
        ImageDimensions::new(5, 4).unwrap(),
        ImageDimensions::new(2, 2).unwrap(),
        ImageDimensions::new(3, 7).unwrap(),
        ImageDimensions::new(8, 6).unwrap(),
        ImageDimensions::new(1, 3).unwrap(),
    ] {
        let (spec_output, prod_output) =
            resize_outputs(&source, source_dimensions, output_dimensions);
        assert_bounded_color_distance(&prod_output, &spec_output, output_dimensions);
    }
}

fn resize_outputs(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> (Vec<u8>, Vec<u8>) {
    let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

    resize_spec_area_into(
        ImageView::<Rgba8>::packed(source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
    );
    resize_prod_area_into(
        ImageView::<Rgba8>::packed(source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
    );

    (spec_output, prod_output)
}

fn assert_bounded_color_distance(left: &[u8], right: &[u8], output_dimensions: ImageDimensions) {
    assert_eq!(left.len(), right.len());
    let mut max_color_distance = 0.0f64;
    let mut color_distance_sum = 0.0f64;
    let mut color_distance_sum_squared = 0.0f64;

    for (left_pixel, right_pixel) in left.chunks_exact(4).zip(right.chunks_exact(4)) {
        let mut channel_distance_sum_squared = 0.0f64;
        for channel in 0..4 {
            let delta = f64::from(left_pixel[channel]) - f64::from(right_pixel[channel]);
            channel_distance_sum_squared += delta * delta;
        }
        let color_distance = channel_distance_sum_squared.sqrt();
        max_color_distance = max_color_distance.max(color_distance);
        color_distance_sum += color_distance;
        color_distance_sum_squared += color_distance * color_distance;
    }

    let pixel_count = left.len() as f64 / 4.0;
    let mean_color_distance = color_distance_sum / pixel_count;
    let rms_color_distance = (color_distance_sum_squared / pixel_count).sqrt();

    assert!(
        max_color_distance <= MAX_COLOR_DISTANCE,
        "output {output_dimensions:?} max color distance {max_color_distance} > {MAX_COLOR_DISTANCE}"
    );
    assert!(
        mean_color_distance <= MAX_MEAN_COLOR_DISTANCE,
        "output {output_dimensions:?} mean color distance {mean_color_distance} > {MAX_MEAN_COLOR_DISTANCE}"
    );
    assert!(
        rms_color_distance <= MAX_RMS_COLOR_DISTANCE,
        "output {output_dimensions:?} rms color distance {rms_color_distance} > {MAX_RMS_COLOR_DISTANCE}"
    );
}

fn patterned_rgba_source(dimensions: ImageDimensions) -> Vec<u8> {
    let mut source = Vec::with_capacity(dimensions.storage_len::<Rgba8>().unwrap());
    for y in 0..dimensions.height_usize() {
        for x in 0..dimensions.width_usize() {
            source.extend_from_slice(&[
                (x * 31 + y * 17) as u8,
                (x * 13 + y * 43) as u8,
                (x * 7 + y * 19) as u8,
                255,
            ]);
        }
    }
    source
}
