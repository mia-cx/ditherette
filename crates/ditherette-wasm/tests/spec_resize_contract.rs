use ditherette_wasm::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8, RowStride, Srgb32},
    spec::{
        contract::{
            error::ErrorCode,
            request::{Anchor, Output, ResizePolicy, ResizeRequest, Source, Support},
        },
        resize::{
            common::alignment::ResizeAnchor, resize, scalar::trilinear::resize_trilinear_into,
        },
    },
};

#[test]
fn trilinear_reads_logical_rows_and_preserves_output_padding() {
    let source_dimensions = ImageDimensions::new(4, 2).unwrap();
    let output_dimensions = ImageDimensions::new(2, 1).unwrap();
    let source = [
        0, 0, 0, 255, 20, 0, 0, 255, 40, 0, 0, 255, 60, 0, 0, 255, 199, 199, 199, 199, 80, 0, 0,
        255, 100, 0, 0, 255, 120, 0, 0, 255, 140, 0, 0, 255,
    ];
    let mut output = [77; 12];
    resize_trilinear_into(
        ImageView::<Rgba8>::new(&source, source_dimensions, RowStride::new(20).unwrap()).unwrap(),
        ImageViewMut::<Rgba8>::new(&mut output, output_dimensions, RowStride::new(12).unwrap())
            .unwrap(),
        ResizeAnchor::Center,
    );
    // The two 2x2 footprints average to 50 and 90. Padding is not image data.
    assert_eq!(output, [50, 0, 0, 255, 90, 0, 0, 255, 77, 77, 77, 77]);
}

const ANCHORS: [Anchor; 9] = [
    Anchor::TopLeft,
    Anchor::Top,
    Anchor::TopRight,
    Anchor::Left,
    Anchor::Center,
    Anchor::Right,
    Anchor::BottomLeft,
    Anchor::Bottom,
    Anchor::BottomRight,
];

fn policies(anchor: Anchor) -> Vec<ResizePolicy> {
    let mut policies = vec![
        ResizePolicy::Nearest { anchor },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear { anchor },
        ResizePolicy::Trilinear { anchor },
    ];
    for support in [Support::Fixed, Support::ScaleAware] {
        policies.extend([
            ResizePolicy::Bicubic { anchor, support },
            ResizePolicy::Lanczos2 { anchor, support },
            ResizePolicy::Lanczos3 { anchor, support },
        ]);
    }
    policies
}

fn resized(data: &[u8], source: (u32, u32), output: (u32, u32), policy: ResizePolicy) -> Vec<u8> {
    resize(ResizeRequest {
        version: 1,
        source: Source {
            data,
            width: source.0,
            height: source.1,
        },
        output: Output {
            width: output.0,
            height: output.1,
            resize: policy,
        },
    })
    .unwrap()
    .into_vec()
}

#[test]
fn every_recipe_preserves_nonconstant_identity_for_all_anchors() {
    let source = [
        0, 7, 250, 10, 211, 30, 17, 254, 75, 125, 42, 0, 17, 199, 67, 131,
    ];
    for anchor in ANCHORS {
        for policy in policies(anchor) {
            assert_eq!(
                resized(&source, (2, 2), (2, 2), policy),
                source,
                "{policy:?}"
            );
        }
    }
}

#[test]
fn every_recipe_preserves_single_pixel_magnification_for_all_anchors() {
    for anchor in ANCHORS {
        for policy in policies(anchor) {
            assert_eq!(
                resized(&[17, 121, 219, 23], (1, 1), (3, 2), policy),
                [17, 121, 219, 23].repeat(6),
                "{policy:?}"
            );
        }
    }
}

#[test]
fn centered_filters_average_a_symmetric_pair_while_nearest_selects_one() {
    let source = [20, 30, 40, 0, 220, 170, 80, 254];
    for policy in policies(Anchor::Center) {
        let expected = if matches!(policy, ResizePolicy::Nearest { .. }) {
            [220, 170, 80, 254]
        } else {
            [120, 100, 60, 127]
        };
        assert_eq!(
            resized(&source, (2, 1), (1, 1), policy),
            expected,
            "{policy:?}"
        );
    }
}

#[test]
fn nearest_requests_preserve_explicit_anchor_mapping() {
    let source: Vec<_> = [10, 20, 30, 40]
        .into_iter()
        .flat_map(|r| [r, 0, 0, 255])
        .collect();
    for (anchor, expected) in [
        (Anchor::TopLeft, [10, 20, 30]),
        (Anchor::Center, [10, 30, 40]),
        (Anchor::BottomRight, [20, 30, 40]),
    ] {
        let output = resized(&source, (4, 1), (3, 1), ResizePolicy::Nearest { anchor });
        assert_eq!(
            output.chunks_exact(4).map(|p| p[0]).collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn cubic_support_policy_changes_nonconstant_minification() {
    let source = [0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 240, 0, 0, 255];
    // Fixed Catmull-Rom weights [-1,9,9,-1]/16 give -15, clipped to zero.
    // Scale-aware clamped-tail weights sum to 1061/1024; all weights sum to 4.
    for (support, expected) in [(Support::Fixed, 0), (Support::ScaleAware, 62)] {
        let result = resized(
            &source,
            (4, 1),
            (1, 1),
            ResizePolicy::Bicubic {
                anchor: Anchor::Center,
                support,
            },
        );
        assert_eq!(result, [expected, 0, 0, 255]);
    }
}

fn trilinear_red(red: &[u8], source: (u32, u32), output: (u32, u32)) -> Vec<u8> {
    let source_data: Vec<_> = red.iter().flat_map(|&r| [r, 0, 0, 255]).collect();
    resized(
        &source_data,
        source,
        output,
        ResizePolicy::Trilinear {
            anchor: Anchor::Center,
        },
    )
    .chunks_exact(4)
    .map(|p| p[0])
    .collect()
}

#[test]
fn fractional_lod_blends_quantized_triangle_outputs() {
    // The original triangle gives [0, 0, 175]. The area mip is [0, 120],
    // whose magnification gives [0, 60, 120]. Blend by log2(4/3), then round.
    assert_eq!(trilinear_red(&[0, 0, 0, 240], (4, 1), (3, 1)), [0, 25, 152]);
}

#[test]
fn odd_dimensions_use_ceil_halved_area_mips_and_byte_rounding() {
    // 5 -> 3 gives mip [0, 0, 153]. The original triangle gives [0, 0, 159].
    // Blend by log2(5/3). Each constant row has the same horizontal result.
    assert_eq!(
        trilinear_red(&[0, 0, 0, 0, 255].repeat(3), (5, 3), (3, 2)),
        [0, 0, 155, 0, 0, 155]
    );
    // Round 254 * 3/5 to 152 before the next 3 -> 2 reduction.
    // A single area reduction would instead produce 102.
    assert_eq!(trilinear_red(&[0, 0, 0, 0, 254], (5, 1), (2, 1)), [0, 101]);
}

#[test]
fn anisotropic_lod_uses_the_minifying_axis() {
    // Width minifies by two while height magnifies. LOD=1 selects [50, 120].
    // Direct source triangle filtering would instead start at 38.
    assert_eq!(
        trilinear_red(&[0, 100, 0, 240], (4, 1), (2, 3)),
        [50, 120, 50, 120, 50, 120]
    );
}

#[test]
fn float_mip_outputs_keep_fractional_samples_until_the_final_blend() {
    let source = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.5, 0.25];
    let mut output = [0.0; 9];
    resize_trilinear_into(
        ImageView::<Srgb32>::packed(&source, ImageDimensions::new(4, 1).unwrap()).unwrap(),
        ImageViewMut::<Srgb32>::packed(&mut output, ImageDimensions::new(3, 1).unwrap()).unwrap(),
        ResizeAnchor::Center,
    );
    let blend = (4.0_f64 / 3.0).log2();
    let last = f64::from((8.0 / 11.0) as f32) * (1.0 - blend) + 0.5 * blend;
    let expected = [
        0.0,
        0.0,
        0.0,
        0.25 * blend,
        0.125 * blend,
        0.0625 * blend,
        last,
        last / 2.0,
        last / 4.0,
    ];
    for (actual, expected) in output.into_iter().zip(expected) {
        assert!((f64::from(actual) - expected).abs() < 1e-6);
    }
}

#[test]
fn reference_call_rejects_malformed_storage_and_output_before_dispatch() {
    let mut request = ResizeRequest {
        version: 1,
        source: Source {
            width: 1,
            height: 1,
            data: &[0, 0, 0],
        },
        output: Output {
            width: 1,
            height: 1,
            resize: ResizePolicy::Area {},
        },
    };
    assert_eq!(resize(request).unwrap_err().code, ErrorCode::InvalidImage);
    request.source.data = &[0, 0, 0, 255];
    request.output.width = 0;
    assert_eq!(
        resize(request).unwrap_err().code,
        ErrorCode::InvalidSettings
    );
}
