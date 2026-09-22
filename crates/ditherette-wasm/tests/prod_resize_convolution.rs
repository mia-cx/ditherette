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
fn fixed_lanczos3_separable_shrink_stays_within_one_rgba_level_of_frozen_spec() {
    for (sw, sh, ow, oh) in [(16, 14, 8, 7), (17, 13, 12, 10), (128, 80, 64, 73)] {
        let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
        let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
        let mut source = patterned_rgba_source(source_dimensions);
        for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
            pixel[3] = (index * 37) as u8;
        }
        for (spec_anchor, prod_anchor) in anchors() {
            let mut expected = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
            let mut actual = expected.clone();
            resize_spec_lanczos3_into(
                ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
                ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
                spec_anchor,
                SpecSupportPolicy::Fixed,
            );
            resize_prod_lanczos3_into(
                ImageView::packed(&source, source_dimensions).unwrap(),
                ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
                prod_anchor,
                ProdSupportPolicy::Fixed,
            );
            assert!(
                actual
                    .iter()
                    .zip(&expected)
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "{sw}x{sh}->{ow}x{oh}, {prod_anchor:?}"
            );
        }
    }
}

#[test]
fn raw_fixed_lanczos3_scratch_has_a_bounded_fixture_delta() {
    let source_dimensions = ImageDimensions::new(128, 80).unwrap();
    let output_dimensions = ImageDimensions::new(64, 73).unwrap();
    let mut source = patterned_rgba_source(source_dimensions);
    for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
        pixel[3] = (index * 37) as u8;
    }
    let mut expected = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let mut actual = expected.clone();
    resize_spec_lanczos3_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
        SpecResizeAnchor::Center,
        SpecSupportPolicy::Fixed,
    );
    resize_prod_lanczos3_into(
        ImageView::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
        ProdResizeAnchor::Center,
        ProdSupportPolicy::Fixed,
    );
    let changed_channels = actual.iter().zip(&expected).filter(|(a, b)| a != b).count();
    let changed_pixels = actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    let max_delta = actual
        .iter()
        .zip(&expected)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    assert_eq!(changed_pixels, 708);
    assert_eq!(changed_channels, 708);
    assert_eq!(max_delta, 1);
    resize_prod_lanczos3_rows_into(
        ImageView::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
        output_dimensions,
        0,
        ProdResizeAnchor::Center,
        ProdSupportPolicy::Fixed,
    );
    let changed_channels = actual.iter().zip(&expected).filter(|(a, b)| a != b).count();
    let changed_pixels = actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    let max_delta = actual
        .iter()
        .zip(&expected)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap();
    // The row-band path retains PR181's normalized horizontal arithmetic.
    assert_eq!(changed_pixels, 396);
    assert_eq!(changed_channels, 396);
    assert_eq!(max_delta, 1);
}

#[test]
fn raw_fixed_lanczos3_scratch_preserves_uniform_alpha_exactly() {
    let source_dimensions = ImageDimensions::new(127, 95).unwrap();
    let output_dimensions = ImageDimensions::new(80, 60).unwrap();
    for alpha in [0, 1, 63, 127, 191, 254, 255] {
        let mut source = patterned_rgba_source(source_dimensions);
        for pixel in source.chunks_exact_mut(4) {
            pixel[3] = alpha;
        }
        let mut output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        resize_prod_lanczos3_into(
            ImageView::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::packed(&mut output, output_dimensions).unwrap(),
            ProdResizeAnchor::Center,
            ProdSupportPolicy::Fixed,
        );
        assert!(output.chunks_exact(4).all(|pixel| pixel[3] == alpha));
    }
}

#[test]
fn opaque_lanczos3_keeps_direct_and_separable_rgb_bytes() {
    for (sw, sh, ow, oh) in [
        (32, 24, 8, 6),   // direct 25%
        (32, 24, 16, 12), // x-then-y 50%
        (32, 24, 24, 18), // x-then-y 75%
    ] {
        let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
        let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
        let opaque = patterned_rgba_source(source_dimensions);
        let mut translucent = opaque.clone();
        for (index, pixel) in translucent.chunks_exact_mut(4).enumerate() {
            pixel[3] = (index * 37) as u8;
        }

        let mut opaque_output = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
        let mut translucent_output = opaque_output.clone();
        resize_prod_lanczos3_into(
            ImageView::packed(&opaque, source_dimensions).unwrap(),
            ImageViewMut::packed(&mut opaque_output, output_dimensions).unwrap(),
            ProdResizeAnchor::Center,
            ProdSupportPolicy::Fixed,
        );
        resize_prod_lanczos3_into(
            ImageView::packed(&translucent, source_dimensions).unwrap(),
            ImageViewMut::packed(&mut translucent_output, output_dimensions).unwrap(),
            ProdResizeAnchor::Center,
            ProdSupportPolicy::Fixed,
        );

        for (opaque_pixel, translucent_pixel) in opaque_output
            .chunks_exact(4)
            .zip(translucent_output.chunks_exact(4))
        {
            assert_eq!(&opaque_pixel[..3], &translucent_pixel[..3]);
            assert_eq!(opaque_pixel[3], u8::MAX);
        }
    }
}

#[test]
fn nonopaque_direct_lanczos3_still_matches_frozen_spec() {
    let source_dimensions = ImageDimensions::new(32, 24).unwrap();
    let output_dimensions = ImageDimensions::new(8, 6).unwrap();
    let mut source = patterned_rgba_source(source_dimensions);
    for (index, pixel) in source.chunks_exact_mut(4).enumerate() {
        pixel[3] = (index * 37) as u8;
    }
    let mut expected = vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
    let mut actual = expected.clone();

    resize_spec_lanczos3_into(
        ImageView::<Rgba8>::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::<Rgba8>::packed(&mut expected, output_dimensions).unwrap(),
        SpecResizeAnchor::Center,
        SpecSupportPolicy::Fixed,
    );
    resize_prod_lanczos3_into(
        ImageView::packed(&source, source_dimensions).unwrap(),
        ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
        ProdResizeAnchor::Center,
        ProdSupportPolicy::Fixed,
    );

    assert_eq!(actual, expected);
}

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
        let opaque = patterned_rgba_source(source_dimensions);
        let mut nonopaque = opaque.clone();
        for (index, pixel) in nonopaque.chunks_exact_mut(4).enumerate() {
            pixel[3] = (index * 37) as u8;
        }

        for (source_kind, source) in [("opaque", opaque), ("nonopaque", nonopaque)] {
            for (spec_anchor, prod_anchor) in anchors() {
                for (spec_policy, prod_policy) in support_policies() {
                    let mut spec_output =
                        vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];
                    let mut prod_output =
                        vec![0; output_dimensions.storage_len::<Rgba8>().unwrap()];

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
                        "{source_kind} output {output_dimensions:?}, anchor {spec_anchor:?}, policy {spec_policy:?}"
                    );
                }
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
