//! Historical S19 experiment conformance, separate from landed production nearest.

use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Oklab32, PaletteIndex8, Rgba8, RowStride},
    prod::resize::{
        common::alignment::ResizeAnchor as ProdAnchor, scalar::nearest_incremental as nearest,
    },
    spec::resize::{common::alignment::ResizeAnchor as SpecAnchor, scalar::nearest as oracle},
};

const ANCHORS: [(SpecAnchor, ProdAnchor); 9] = [
    (SpecAnchor::TopLeft, ProdAnchor::TopLeft),
    (SpecAnchor::Top, ProdAnchor::Top),
    (SpecAnchor::TopRight, ProdAnchor::TopRight),
    (SpecAnchor::Left, ProdAnchor::Left),
    (SpecAnchor::Center, ProdAnchor::Center),
    (SpecAnchor::Right, ProdAnchor::Right),
    (SpecAnchor::BottomLeft, ProdAnchor::BottomLeft),
    (SpecAnchor::Bottom, ProdAnchor::Bottom),
    (SpecAnchor::BottomRight, ProdAnchor::BottomRight),
];

#[test]
fn rgba_bytes_match_all_anchors_shapes_and_independent_row_strides() {
    let shapes = [(1, 1), (1, 7), (7, 1), (2, 3), (4, 4), (7, 5), (13, 9)];
    for (sw, sh) in shapes {
        for (ow, oh) in shapes {
            for source_padding in [0, 3] {
                for output_padding in [0, 5] {
                    let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
                    let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
                    let source_stride = sw as usize * 4 + source_padding;
                    let output_stride = ow as usize * 4 + output_padding;
                    let source: Vec<u8> = (0..source_stride * sh as usize)
                        .map(|i| (i.wrapping_mul(73).wrapping_add(19) % 256) as u8)
                        .collect();
                    let original = source.clone();
                    for (spec_anchor, prod_anchor) in ANCHORS {
                        let mut expected = vec![203; output_stride * oh as usize];
                        let mut actual = expected.clone();
                        let view = ImageView::<Rgba8>::new(
                            &source,
                            source_dimensions,
                            RowStride::new(source_stride).unwrap(),
                        )
                        .unwrap();
                        oracle::resize_nearest_into(
                            view,
                            ImageViewMut::new(
                                &mut expected,
                                output_dimensions,
                                RowStride::new(output_stride).unwrap(),
                            )
                            .unwrap(),
                            spec_anchor,
                        );
                        nearest::resize_nearest_into(
                            view,
                            ImageViewMut::new(
                                &mut actual,
                                output_dimensions,
                                RowStride::new(output_stride).unwrap(),
                            )
                            .unwrap(),
                            prod_anchor,
                        );
                        assert_eq!(actual, expected, "{sw}x{sh} -> {ow}x{oh}, {prod_anchor:?}");
                        for row in actual.chunks_exact(output_stride) {
                            assert!(row[ow as usize * 4..].iter().all(|&byte| byte == 203));
                        }
                        assert_eq!(source, original);
                    }
                }
            }
        }
    }
}

#[test]
fn generic_float_bits_and_palette_indices_are_copied_without_arithmetic() {
    let source_dimensions = ImageDimensions::new(2, 1).unwrap();
    let output_dimensions = ImageDimensions::new(5, 1).unwrap();
    let source = [
        f32::from_bits(0x7fc0_1234),
        -0.0,
        f32::INFINITY,
        0.25,
        -0.75,
        f32::NEG_INFINITY,
    ];
    for (spec_anchor, prod_anchor) in ANCHORS {
        let mut expected = [0.0; 15];
        let mut actual = [0.0; 15];
        oracle::resize_nearest_into(
            ImageView::<Oklab32>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
            spec_anchor,
        );
        nearest::resize_nearest_into(
            ImageView::<Oklab32>::packed(&source, source_dimensions).unwrap(),
            ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
            prod_anchor,
        );
        assert_eq!(actual.map(f32::to_bits), expected.map(f32::to_bits));

        let mut expected = [0; 5];
        let mut actual = [0; 5];
        oracle::resize_nearest_into(
            ImageView::<PaletteIndex8>::packed(&[0, 255], source_dimensions).unwrap(),
            ImageViewMut::packed(&mut expected, output_dimensions).unwrap(),
            spec_anchor,
        );
        nearest::resize_nearest_into(
            ImageView::<PaletteIndex8>::packed(&[0, 255], source_dimensions).unwrap(),
            ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
            prod_anchor,
        );
        assert_eq!(actual, expected);
    }
}

#[cfg(feature = "bench-subjects")]
#[test]
fn registry_calls_reference_restored_production_and_historical_alias() {
    use ditherette_bench_api::{BenchSubject, ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams};
    let subjects = ditherette_wasm::bench_subjects::bench_subjects();
    let ids = [
        (
            "spec:resize:nearest:scalar",
            "spec/resize/scalar/nearest.rs",
        ),
        (
            "prod:resize:nearest:scalar",
            "prod/resize/scalar/nearest/mod.rs",
        ),
        (
            "candidate:resize:nearest:legacy",
            "prod/resize/scalar/nearest/mod.rs",
        ),
    ];
    for (id, source_file) in ids {
        let subject = subjects
            .iter()
            .find(|subject| subject.descriptor().id.as_str() == id)
            .unwrap();
        let BenchSubject::Resize(subject) = subject else {
            panic!("nearest requires a resize adapter");
        };
        assert!(subject.descriptor.source_file.ends_with(source_file));
        assert_eq!(
            subject
                .descriptor
                .default_oracle
                .as_ref()
                .map(|id| id.as_str()),
            if id.starts_with("spec:") {
                None
            } else {
                Some("spec:resize:nearest:scalar")
            }
        );
        let mut output = [0; 8];
        subject
            .resize_u8_rgba(
                ResizeInputU8Rgba {
                    data: &[10, 11, 12, 0, 20, 21, 22, 255, 30, 31, 32, 127],
                    width: 3,
                    height: 1,
                    row_stride_elements: 12,
                },
                ResizeOutputU8Rgba {
                    data: &mut output,
                    width: 2,
                    height: 1,
                    row_stride_elements: 8,
                },
                &ResizeParams::default(),
            )
            .unwrap();
        assert_eq!(output, [10, 11, 12, 0, 30, 31, 32, 127], "{id}");
    }
}
