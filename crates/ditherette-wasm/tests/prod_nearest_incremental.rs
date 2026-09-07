//! Exact candidate conformance against the frozen nearest kernel.

use ditherette_wasm::{
    image::{
        ImageDimensions, ImageFormat, ImageView, ImageViewMut, Oklab32, PaletteIndex8, Rgba8,
        RowStride,
    },
    prod::resize::{common::alignment::ResizeAnchor as ProdAnchor, scalar::nearest_incremental},
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

fn compare_matrix<F: ImageFormat + Copy>(
    pattern: &[F::Storage],
    sentinel: F::Storage,
    bits: impl Fn(F::Storage) -> u32,
) {
    let shapes = [(1, 1), (1, 7), (7, 1), (2, 3), (4, 4), (7, 5), (13, 9)];
    for (sw, sh) in shapes {
        for (ow, oh) in shapes {
            for source_padding in [0, 3] {
                for output_padding in [0, 5] {
                    let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
                    let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
                    let source_stride = sw as usize * F::CHANNEL_COUNT + source_padding;
                    let output_stride = ow as usize * F::CHANNEL_COUNT + output_padding;
                    let source: Vec<_> = pattern
                        .iter()
                        .copied()
                        .cycle()
                        .take(source_stride * sh as usize)
                        .collect();
                    let original: Vec<_> = source.iter().copied().map(&bits).collect();
                    for (spec_anchor, prod_anchor) in ANCHORS {
                        let mut expected = vec![sentinel; output_stride * oh as usize];
                        let mut actual = expected.clone();
                        let view = ImageView::<F>::new(
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
                        nearest_incremental::resize_nearest_into(
                            view,
                            ImageViewMut::new(
                                &mut actual,
                                output_dimensions,
                                RowStride::new(output_stride).unwrap(),
                            )
                            .unwrap(),
                            prod_anchor,
                        );
                        assert_eq!(actual.iter().copied().map(&bits).collect::<Vec<_>>(), expected.iter().copied().map(&bits).collect::<Vec<_>>(), "{} {sw}x{sh} -> {ow}x{oh}, {prod_anchor:?}, padding {source_padding}/{output_padding}", F::NAME);
                        for row in actual.chunks_exact(output_stride) {
                            assert!(row[ow as usize * F::CHANNEL_COUNT..]
                                .iter()
                                .all(|&value| bits(value) == bits(sentinel)));
                        }
                    }
                    assert_eq!(
                        source.iter().copied().map(&bits).collect::<Vec<_>>(),
                        original
                    );
                }
            }
        }
    }
}

#[test]
fn rgba_bytes_match_all_anchors_shapes_and_strides() {
    let pattern: Vec<u8> = (0..256).map(|i| ((i * 73 + 19) % 256) as u8).collect();
    compare_matrix::<Rgba8>(&pattern, 203, u32::from);
}

#[test]
fn float_payload_bits_match_all_anchors_shapes_and_strides() {
    compare_matrix::<Oklab32>(
        &[
            f32::from_bits(0x7fc0_1234),
            -0.0,
            f32::INFINITY,
            0.25,
            -0.75,
            f32::NEG_INFINITY,
        ],
        f32::from_bits(0x7fc0_5678),
        f32::to_bits,
    );
}

#[test]
fn palette_indices_match_all_anchors_shapes_and_strides() {
    compare_matrix::<PaletteIndex8>(&[0, 255, 1, 127, 3, 252, 99], 203, u32::from);
}

#[cfg(feature = "bench-subjects")]
#[test]
fn registry_calls_incremental_candidate_with_frozen_oracle() {
    use ditherette_bench_api::{BenchSubject, ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams};

    let subjects = ditherette_wasm::bench_subjects::bench_subjects();
    let candidate = subjects
        .iter()
        .find(|subject| subject.descriptor().id.as_str() == "candidate:resize:nearest:incremental")
        .unwrap();
    let BenchSubject::Resize(candidate) = candidate else {
        panic!("nearest candidate requires a resize adapter");
    };
    assert_eq!(
        candidate
            .descriptor
            .default_oracle
            .as_ref()
            .unwrap()
            .as_str(),
        "spec:resize:nearest:scalar"
    );
    assert!(candidate
        .descriptor
        .source_file
        .ends_with("prod/resize/scalar/nearest_incremental.rs"));
    let mut output = [0; 8];
    candidate
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
    assert_eq!(output, [10, 11, 12, 0, 30, 31, 32, 127]);
}
