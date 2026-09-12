use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::scalar::{
        bicubic::{
            resize_bicubic_rgba8_into as resize_prod_bicubic_into,
            resize_bicubic_rgba8_rows_into as resize_prod_bicubic_rows_into,
        },
        convolution::{ResizeAnchor as ProdResizeAnchor, SupportPolicy as ProdSupportPolicy},
        lanczos::{
            resize_lanczos2_rgba8_into as resize_prod_lanczos2_into,
            resize_lanczos2_rgba8_rows_into as resize_prod_lanczos2_rows_into,
            resize_lanczos3_rgba8_into as resize_prod_lanczos3_into,
            resize_lanczos3_rgba8_rows_into as resize_prod_lanczos3_rows_into,
        },
    },
    spec::resize::{
        common::alignment::ResizeAnchor as SpecResizeAnchor,
        scalar::{
            bicubic::resize_bicubic_into as resize_spec_bicubic_into,
            convolution::SupportPolicy as SpecSupportPolicy,
            lanczos::{
                resize_lanczos2_into as resize_spec_lanczos2_into,
                resize_lanczos3_into as resize_spec_lanczos3_into,
            },
        },
    },
};

#[test]
fn prod_bicubic_matches_spec_for_anchor_and_policy_matrix() {
    assert_matches_spec(
        |source, spec_output, prod_output, spec_anchor, prod_anchor, spec_policy, prod_policy| {
            resize_spec_bicubic_into(source, spec_output, spec_anchor, spec_policy);
            resize_prod_bicubic_into(source, prod_output, prod_anchor, prod_policy);
        },
    );
}

#[test]
fn prod_convolution_row_ranges_match_full_output() {
    let source_dimensions = ImageDimensions::new(128, 80).unwrap();
    let output_dimensions = ImageDimensions::new(64, 73).unwrap();

    assert_row_ranges_match_full(
        source_dimensions,
        output_dimensions,
        |source, output, policy| {
            resize_prod_bicubic_into(source, output, ProdResizeAnchor::Center, policy);
        },
        |source, output, y_start, policy| {
            resize_prod_bicubic_rows_into(
                source,
                output,
                output_dimensions,
                y_start,
                ProdResizeAnchor::Center,
                policy,
            );
        },
    );

    assert_row_ranges_match_full(
        source_dimensions,
        output_dimensions,
        |source, output, policy| {
            resize_prod_lanczos2_into(source, output, ProdResizeAnchor::Center, policy);
        },
        |source, output, y_start, policy| {
            resize_prod_lanczos2_rows_into(
                source,
                output,
                output_dimensions,
                y_start,
                ProdResizeAnchor::Center,
                policy,
            );
        },
    );

    assert_row_ranges_match_full(
        source_dimensions,
        output_dimensions,
        |source, output, policy| {
            resize_prod_lanczos3_into(source, output, ProdResizeAnchor::Center, policy);
        },
        |source, output, y_start, policy| {
            resize_prod_lanczos3_rows_into(
                source,
                output,
                output_dimensions,
                y_start,
                ProdResizeAnchor::Center,
                policy,
            );
        },
    );
}

#[test]
fn prod_lanczos2_matches_spec_for_anchor_and_policy_matrix() {
    assert_matches_spec(
        |source, spec_output, prod_output, spec_anchor, prod_anchor, spec_policy, prod_policy| {
            resize_spec_lanczos2_into(source, spec_output, spec_anchor, spec_policy);
            resize_prod_lanczos2_into(source, prod_output, prod_anchor, prod_policy);
        },
    );
}

#[test]
fn prod_lanczos3_matches_spec_for_anchor_and_policy_matrix() {
    assert_matches_spec(
        |source, spec_output, prod_output, spec_anchor, prod_anchor, spec_policy, prod_policy| {
            resize_spec_lanczos3_into(source, spec_output, spec_anchor, spec_policy);
            resize_prod_lanczos3_into(source, prod_output, prod_anchor, prod_policy);
        },
    );
}

fn assert_matches_spec(
    resize: impl Fn(
        ImageView<'_, Rgba8>,
        ImageViewMut<'_, Rgba8>,
        ImageViewMut<'_, Rgba8>,
        SpecResizeAnchor,
        ProdResizeAnchor,
        SpecSupportPolicy,
        ProdSupportPolicy,
    ),
) {
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
            ImageDimensions::new(5, 5).unwrap(),
            ImageDimensions::new(5, 5).unwrap(),
        ),
    ] {
        let source = patterned_rgba_source(source_dimensions);

        for (spec_anchor, prod_anchor) in anchors() {
            for (spec_policy, prod_policy) in support_policies() {
                let mut spec_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
                let mut prod_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

                resize(
                    ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
                    ImageViewMut::<Rgba8>::packed(&mut spec_output, output_dimensions).unwrap(),
                    ImageViewMut::<Rgba8>::packed(&mut prod_output, output_dimensions).unwrap(),
                    spec_anchor,
                    prod_anchor,
                    spec_policy,
                    prod_policy,
                );

                assert_eq!(
                    prod_output, spec_output,
                    "output {output_dimensions:?}, anchor {spec_anchor:?}, policy {spec_policy:?}"
                );
            }
        }
    }
}

fn assert_row_ranges_match_full(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    resize_full: impl Fn(ImageView<'_, Rgba8>, ImageViewMut<'_, Rgba8>, ProdSupportPolicy),
    resize_row: impl Fn(ImageView<'_, Rgba8>, ImageViewMut<'_, Rgba8>, u32, ProdSupportPolicy),
) {
    let source = patterned_rgba_source(source_dimensions);

    for (_, policy) in support_policies() {
        let mut full_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        resize_full(
            ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::<Rgba8>::packed(&mut full_output, output_dimensions).unwrap(),
            policy,
        );

        let mut row_range_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let output_row_len = output_dimensions.width_usize() * 4;
        let mut y_start = 0;
        while y_start < output_dimensions.height() {
            let y_end = (y_start + 7).min(output_dimensions.height());
            let byte_start = y_start as usize * output_row_len;
            let byte_end = y_end as usize * output_row_len;
            resize_row(
                ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
                ImageViewMut::<Rgba8>::packed(
                    &mut row_range_output[byte_start..byte_end],
                    ImageDimensions::new(output_dimensions.width(), y_end - y_start).unwrap(),
                )
                .unwrap(),
                y_start,
                policy,
            );
            y_start = y_end;
        }

        assert_eq!(row_range_output, full_output, "policy {policy:?}");
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

fn support_policies() -> [(SpecSupportPolicy, ProdSupportPolicy); 2] {
    [
        (SpecSupportPolicy::Fixed, ProdSupportPolicy::Fixed),
        (SpecSupportPolicy::ScaleAware, ProdSupportPolicy::ScaleAware),
    ]
}

fn patterned_rgba_source(dimensions: ImageDimensions) -> Vec<u8> {
    let mut source = Vec::with_capacity(dimensions.storage_len::<Rgba8>().unwrap());
    for y in 0..dimensions.height_usize() {
        for x in 0..dimensions.width_usize() {
            source.extend_from_slice(&[
                (x * 29 + y * 17) as u8,
                (x * 11 + y * 47) as u8,
                (x * 7 + y * 23) as u8,
                255,
            ]);
        }
    }
    source
}
