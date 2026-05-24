use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::scalar::nearest::{
        alignment::ResizeAnchor as ProdResizeAnchor,
        resize_nearest_rgba8_into as resize_prod_nearest_into,
        resize_nearest_rgba8_rows_with_plan_into, NearestResizePlan,
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecResizeAnchor,
        scalar::nearest::resize_nearest_into as resize_spec_nearest_into,
    },
};

#[test]
fn prod_nearest_matches_spec_for_anchor_matrix() {
    assert_prod_matches_spec(
        ImageDimensions::new(4, 3).unwrap(),
        ImageDimensions::new(7, 5).unwrap(),
    );
}

#[test]
fn prod_nearest_exact_upscale_matches_spec_for_anchor_matrix() {
    assert_prod_matches_spec(
        ImageDimensions::new(4, 3).unwrap(),
        ImageDimensions::new(8, 9).unwrap(),
    );
}

#[test]
fn prod_nearest_row_ranges_match_full_fast_paths() {
    for (source_dimensions, output_dimensions) in [
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(4, 5).unwrap(),
        ),
        (
            ImageDimensions::new(8, 8).unwrap(),
            ImageDimensions::new(4, 4).unwrap(),
        ),
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(8, 6).unwrap(),
        ),
        (
            ImageDimensions::new(100, 10).unwrap(),
            ImageDimensions::new(95, 9).unwrap(),
        ),
        (
            ImageDimensions::new(4, 3).unwrap(),
            ImageDimensions::new(7, 5).unwrap(),
        ),
    ] {
        assert_row_ranges_match_full(source_dimensions, output_dimensions);
    }
}

fn assert_prod_matches_spec(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) {
    let source = numbered_rgba_source(source_dimensions);

    for (spec_anchor, prod_anchor) in anchors() {
        let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

        resize_spec_nearest_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
            spec_anchor,
        );
        resize_prod_nearest_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
            prod_anchor,
        );

        assert_eq!(prod_output, spec_output, "anchor {spec_anchor:?}");
    }
}

fn assert_row_ranges_match_full(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) {
    let source = numbered_rgba_source(source_dimensions);
    let mut full_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    resize_prod_nearest_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut full_output, output_dimensions).unwrap(),
        ProdResizeAnchor::Center,
    );

    let mut row_range_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let plan = NearestResizePlan::new(
        source_dimensions,
        output_dimensions,
        ProdResizeAnchor::Center,
    );
    let output_row_len =
        output_dimensions.storage_len::<Rgba8>().unwrap() / output_dimensions.height_usize();
    let mut y_start = 0;
    while y_start < output_dimensions.height() {
        let y_end = (y_start + 2).min(output_dimensions.height());
        let byte_start = y_start as usize * output_row_len;
        let byte_end = y_end as usize * output_row_len;
        resize_nearest_rgba8_rows_with_plan_into(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(
                &mut row_range_output[byte_start..byte_end],
                ImageDimensions::new(output_dimensions.width(), y_end - y_start).unwrap(),
            )
            .unwrap(),
            &plan,
            y_start,
        );
        y_start = y_end;
    }

    assert_eq!(
        row_range_output, full_output,
        "{source_dimensions:?} -> {output_dimensions:?}"
    );
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
        source.extend_from_slice(&(index as u32).to_le_bytes());
    }
    source
}
