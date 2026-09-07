//! S22 copied baselines compared against independently executed frozen functions.

use ditherette_wasm::{
    image::{
        ImageDimensions, ImageFormat, ImageView, ImageViewMut, LinearRgba32, Oklab32,
        PaletteIndex8, Rgb8, Rgba8, RowStride,
    },
    prod::resize::{common as production_common, scalar as production},
    spec::resize::{common as frozen_common, scalar as frozen},
};

const ANCHORS: [(
    frozen_common::alignment::ResizeAnchor,
    production_common::alignment::ResizeAnchor,
); 9] = {
    use frozen_common::alignment::ResizeAnchor as S;
    use production_common::alignment::ResizeAnchor as P;
    [
        (S::TopLeft, P::TopLeft),
        (S::Top, P::Top),
        (S::TopRight, P::TopRight),
        (S::Left, P::Left),
        (S::Center, P::Center),
        (S::Right, P::Right),
        (S::BottomLeft, P::BottomLeft),
        (S::Bottom, P::Bottom),
        (S::BottomRight, P::BottomRight),
    ]
};

fn compare<F: ImageFormat + Copy>(
    sample: impl Fn(usize) -> F::Storage,
    bits: impl Fn(F::Storage) -> u32,
) where
    F::Storage: production_common::sample::ResizeSample + frozen_common::sample::ResizeSample,
{
    for ((sw, sh), (ow, oh)) in [
        ((1, 1), (1, 1)),
        ((1, 5), (7, 1)),
        ((7, 1), (1, 5)),
        ((3, 5), (3, 5)),
        ((3, 5), (7, 9)),
        ((11, 7), (3, 2)),
        ((7, 3), (2, 9)),
        ((7, 5), (7, 2)),
        ((7, 5), (3, 5)),
    ] {
        for source_padding in [0, 3] {
            for output_padding in [0, 5] {
                let source_dimensions = ImageDimensions::new(sw, sh).unwrap();
                let output_dimensions = ImageDimensions::new(ow, oh).unwrap();
                let source_stride = sw as usize * F::CHANNEL_COUNT + source_padding;
                let output_stride = ow as usize * F::CHANNEL_COUNT + output_padding;
                let source: Vec<_> = (0..source_stride * sh as usize).map(&sample).collect();
                let before: Vec<_> = source.iter().copied().map(&bits).collect();
                let source_view = ImageView::<F>::new(
                    &source,
                    source_dimensions,
                    RowStride::new(source_stride).unwrap(),
                )
                .unwrap();
                for (sa, pa) in ANCHORS {
                    for scale_aware in [false, true] {
                        let sp = if scale_aware {
                            frozen::convolution::SupportPolicy::ScaleAware
                        } else {
                            frozen::convolution::SupportPolicy::Fixed
                        };
                        let pp = if scale_aware {
                            production::convolution::SupportPolicy::ScaleAware
                        } else {
                            production::convolution::SupportPolicy::Fixed
                        };
                        for filter in 0..3 {
                            let sentinel = sample(203);
                            let mut expected = vec![sentinel; output_stride * oh as usize];
                            let mut actual = expected.clone();
                            let expected_view = ImageViewMut::<F>::new(
                                &mut expected,
                                output_dimensions,
                                RowStride::new(output_stride).unwrap(),
                            )
                            .unwrap();
                            let actual_view = ImageViewMut::<F>::new(
                                &mut actual,
                                output_dimensions,
                                RowStride::new(output_stride).unwrap(),
                            )
                            .unwrap();
                            match filter {
                                0 => {
                                    frozen::bicubic::resize_bicubic_into(
                                        source_view,
                                        expected_view,
                                        sa,
                                        sp,
                                    );
                                    production::bicubic::resize_bicubic_into(
                                        source_view,
                                        actual_view,
                                        pa,
                                        pp,
                                    );
                                }
                                1 => {
                                    frozen::lanczos::resize_lanczos2_into(
                                        source_view,
                                        expected_view,
                                        sa,
                                        sp,
                                    );
                                    production::lanczos::resize_lanczos2_into(
                                        source_view,
                                        actual_view,
                                        pa,
                                        pp,
                                    );
                                }
                                _ => {
                                    frozen::lanczos::resize_lanczos3_into(
                                        source_view,
                                        expected_view,
                                        sa,
                                        sp,
                                    );
                                    production::lanczos::resize_lanczos3_into(
                                        source_view,
                                        actual_view,
                                        pa,
                                        pp,
                                    );
                                }
                            }
                            assert_eq!(actual.iter().copied().map(&bits).collect::<Vec<_>>(), expected.iter().copied().map(&bits).collect::<Vec<_>>(),
                                "{}, filter {filter}, {sw}x{sh} -> {ow}x{oh}, {pa:?}, scale-aware {scale_aware}", F::NAME);
                            for row in actual.chunks_exact(output_stride) {
                                assert!(row[ow as usize * F::CHANNEL_COUNT..]
                                    .iter()
                                    .all(|&v| bits(v) == bits(sentinel)));
                            }
                        }
                    }
                }
                assert_eq!(
                    source.iter().copied().map(&bits).collect::<Vec<_>>(),
                    before
                );
            }
        }
    }
}

#[test]
fn exact_bytes_cover_every_anchor_support_shape_and_independent_stride() {
    let sample = |i: usize| [0, 255, 1, 254, 127, 128, 17, 239][i % 8];
    compare::<Rgba8>(sample, u32::from);
    compare::<Rgb8>(sample, u32::from);
    compare::<PaletteIndex8>(sample, u32::from);
}

#[test]
fn float_storage_preserves_exact_bits_without_byte_clipping() {
    let sample = |i: usize| [-0.0, -20.5, 0.25, 1.0, 300.25, -0.125, 127.75][i % 7];
    compare::<Oklab32>(sample, f32::to_bits);
    compare::<LinearRgba32>(sample, f32::to_bits);
}

#[test]
fn negative_lobes_are_retained_in_floats_and_clipped_then_rounded_in_bytes() {
    let input = ImageDimensions::new(4, 1).unwrap();
    let output = ImageDimensions::new(8, 1).unwrap();
    let floats: Vec<_> = [0.0, 0.0, 255.0, 255.0]
        .into_iter()
        .flat_map(|v| [v; 3])
        .collect();
    let bytes = [0, 0, 255, 255];
    let mut actual = [0.0; 24];
    let mut rounded = [0; 8];
    production::bicubic::resize_bicubic_into(
        ImageView::<Oklab32>::packed(&floats, input).unwrap(),
        ImageViewMut::packed(&mut actual, output).unwrap(),
        production_common::alignment::ResizeAnchor::Center,
        production::convolution::SupportPolicy::Fixed,
    );
    production::bicubic::resize_bicubic_into(
        ImageView::<PaletteIndex8>::packed(&bytes, input).unwrap(),
        ImageViewMut::packed(&mut rounded, output).unwrap(),
        production_common::alignment::ResizeAnchor::Center,
        production::convolution::SupportPolicy::Fixed,
    );
    assert_eq!(
        actual
            .chunks_exact(3)
            .map(|pixel| pixel[0])
            .collect::<Vec<_>>(),
        [
            0.0,
            -5.9765625,
            -17.9296875,
            51.796875,
            203.203125,
            272.9296875,
            260.9765625,
            255.0
        ]
    );
    assert_eq!(rounded, [0, 0, 0, 52, 203, 255, 255, 255]);
    assert_eq!(
        <u8 as production_common::sample::ResizeSample>::from_f64(0.5),
        1
    );
    assert_eq!(
        <u8 as production_common::sample::ResizeSample>::from_f64(254.5),
        255
    );
}

#[cfg(feature = "bench-subjects")]
#[test]
fn inherited_subjects_are_explicit_candidates_with_frozen_oracles() {
    let subjects = ditherette_wasm::bench_subjects::bench_subjects();
    for (family, variants, file) in [
        (
            "bicubic",
            ["catmull-rom", "catmull-rom-scale-aware"],
            "bicubic",
        ),
        ("lanczos2", ["fixed", "scale-aware"], "lanczos"),
        ("lanczos3", ["fixed", "scale-aware"], "lanczos"),
    ] {
        for variant in variants {
            let id = format!("candidate:resize:{family}:{variant}");
            let subject = subjects
                .iter()
                .find(|subject| subject.descriptor().id.as_str() == id)
                .unwrap();
            let descriptor = subject.descriptor();
            assert!(descriptor
                .source_file
                .ends_with(&format!("{file}_candidate/mod.rs")));
            assert_eq!(
                descriptor.default_oracle.as_ref().unwrap().as_str(),
                format!("spec:resize:{family}:{variant}")
            );
            assert!(!subjects
                .iter()
                .any(|subject| subject.descriptor().id.as_str()
                    == format!("prod:resize:{family}:{variant}")));
        }
    }
}
