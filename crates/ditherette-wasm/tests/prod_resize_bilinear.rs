use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::scalar::bilinear::{
        alignment::ResizeAnchor as ProdResizeAnchor,
        resize_bilinear_rgba8_into as resize_prod_bilinear_into,
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecResizeAnchor,
        scalar::bilinear::resize_bilinear_into as resize_spec_bilinear_into,
    },
};

#[test]
fn prod_bilinear_stays_near_spec_for_anchor_matrix() {
    for (source_dimensions, output_dimensions) in [
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(7, 5).unwrap(),
        ),
        (
            ImageDimensions::new(7, 5).unwrap(),
            ImageDimensions::new(4, 3).unwrap(),
        ),
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(8, 9).unwrap(),
        ),
    ] {
        assert_prod_stays_near_spec(source_dimensions, output_dimensions);
    }
}

fn assert_prod_stays_near_spec(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) {
    let source = numbered_rgba_source(source_dimensions);

    for (spec_anchor, prod_anchor) in anchors() {
        let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

        resize_spec_bilinear_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
            spec_anchor,
        );
        resize_prod_bilinear_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
            prod_anchor,
        );

        assert_bounded_color_distance(&prod_output, &spec_output, 2.0, "anchor {spec_anchor:?}");
    }
}

fn assert_bounded_color_distance(actual: &[u8], expected: &[u8], max_distance: f64, context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context} output length");

    for (pixel_index, (actual_pixel, expected_pixel)) in actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .enumerate()
    {
        let distance = actual_pixel
            .iter()
            .zip(expected_pixel)
            .map(|(actual, expected)| f64::from(actual.abs_diff(*expected)).powi(2))
            .sum::<f64>()
            .sqrt();

        assert!(
            distance <= max_distance,
            "{context} pixel {pixel_index} color distance {distance} exceeded {max_distance}: actual={actual_pixel:?} expected={expected_pixel:?}"
        );
    }
}

fn anchors() -> [(SpecResizeAnchor, ProdResizeAnchor); 9] {
    [
        (SpecResizeAnchor::TopLeft, ProdResizeAnchor::TopLeft),
        (SpecResizeAnchor::Top, ProdResizeAnchor::Top),
        (SpecResizeAnchor::TopRight, ProdResizeAnchor::TopRight),
        (SpecResizeAnchor::Left, ProdResizeAnchor::Left),
        (SpecResizeAnchor::Center, ProdResizeAnchor::Center),
        (SpecResizeAnchor::Right, ProdResizeAnchor::Right),
        (SpecResizeAnchor::BottomLeft, ProdResizeAnchor::BottomLeft),
        (SpecResizeAnchor::Bottom, ProdResizeAnchor::Bottom),
        (SpecResizeAnchor::BottomRight, ProdResizeAnchor::BottomRight),
    ]
}

fn numbered_rgba_source(dimensions: ImageDimensions) -> Vec<u8> {
    let mut source = Vec::with_capacity(dimensions.storage_len::<Rgba8>().unwrap());
    for index in 0..dimensions.pixel_count().unwrap() {
        source.extend_from_slice(&[
            index as u8,
            index.wrapping_mul(3) as u8,
            index.wrapping_mul(7) as u8,
            255,
        ]);
    }
    source
}
