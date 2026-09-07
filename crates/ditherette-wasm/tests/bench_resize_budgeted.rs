#![cfg(feature = "bench-subjects")]

use ditherette_bench_api::{
    ResizeAnchorParam, ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams,
};
use ditherette_wasm::bench_subjects::{bench_subjects, BenchSubject};

const PAIRS: [(&str, &str); 8] = [
    ("area:budgeted", "area:scalar"),
    ("bilinear:budgeted", "bilinear:scalar"),
    ("bicubic:budgeted-fixed", "bicubic:catmull-rom"),
    (
        "bicubic:budgeted-scale-aware",
        "bicubic:catmull-rom-scale-aware",
    ),
    ("lanczos2:budgeted-fixed", "lanczos2:fixed"),
    ("lanczos2:budgeted-scale-aware", "lanczos2:scale-aware"),
    ("lanczos3:budgeted-fixed", "lanczos3:fixed"),
    ("lanczos3:budgeted-scale-aware", "lanczos3:scale-aware"),
];

fn subject<'a>(
    registry: &'a [BenchSubject],
    id: &str,
) -> &'a ditherette_bench_api::ResizeBenchSubject {
    registry
        .iter()
        .find_map(|subject| match subject {
            BenchSubject::Resize(resize) if resize.descriptor.id.to_string() == id => Some(resize),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing {id}"))
}

#[test]
fn registry_points_to_real_adapter_and_matching_oracles() {
    let registry = bench_subjects();
    for (candidate, accepted) in PAIRS {
        let candidate = subject(&registry, &format!("candidate:resize:{candidate}"));
        let accepted = subject(&registry, &format!("prod:resize:{accepted}"));
        assert_eq!(
            candidate.descriptor.default_oracle,
            accepted.descriptor.default_oracle
        );
        assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(&candidate.descriptor.source_file)
            .is_file());
        subject(
            &registry,
            &candidate
                .descriptor
                .default_oracle
                .as_ref()
                .unwrap()
                .to_string(),
        );
    }
}

#[test]
fn matches_landed_subjects_exactly_with_all_anchors_and_alpha() {
    use ResizeAnchorParam::*;
    let registry = bench_subjects();
    // Includes identity, integer fast paths, odd dimensions, single axes, and
    // the >=10,000-pixel scale-aware x-then-y convolution dispatch.
    let shapes = [
        (1, 1, 1, 1),
        (8, 6, 4, 3),
        (3, 2, 9, 6),
        (7, 5, 4, 3),
        (4, 3, 7, 5),
        (7, 5, 7, 3),
        (7, 5, 4, 5),
        (101, 100, 37, 29),
    ];
    for (candidate, accepted) in PAIRS {
        let candidate = subject(&registry, &format!("candidate:resize:{candidate}"));
        let accepted = subject(&registry, &format!("prod:resize:{accepted}"));
        for (sw, sh, ow, oh) in shapes {
            let source: Vec<u8> = (0..sw * sh)
                .flat_map(|i| {
                    [
                        (i * 17) as u8,
                        (i * 31 + 13) as u8,
                        (i * 7 + 251) as u8,
                        [0, 1, 127, 255][i as usize % 4],
                    ]
                })
                .collect();
            let preserved = source.clone();
            for anchor in [
                TopLeft,
                Top,
                TopRight,
                Left,
                Center,
                Right,
                BottomLeft,
                Bottom,
                BottomRight,
            ] {
                let params = ResizeParams {
                    anchor,
                    ..Default::default()
                };
                let mut expected = vec![0; (ow * oh * 4) as usize];
                let mut actual = vec![0xA5; expected.len()];
                for (entry, bytes) in [(accepted, &mut expected), (candidate, &mut actual)] {
                    (entry.resize_u8_rgba)(
                        ResizeInputU8Rgba {
                            data: &source,
                            width: sw,
                            height: sh,
                            row_stride_elements: sw as usize * 4,
                        },
                        ResizeOutputU8Rgba {
                            data: bytes,
                            width: ow,
                            height: oh,
                            row_stride_elements: ow as usize * 4,
                        },
                        &params,
                    )
                    .unwrap();
                }
                assert_eq!(
                    actual, expected,
                    "{} {sw}x{sh}->{ow}x{oh} {anchor:?}",
                    candidate.descriptor.id
                );
                assert_eq!(source, preserved);
            }
        }
    }
}

#[test]
fn invalid_views_return_adapter_errors_without_writing_output() {
    let registry = bench_subjects();
    for (candidate, _) in PAIRS {
        let entry = subject(&registry, &format!("candidate:resize:{candidate}"));
        let mut output = [91; 4];
        assert!((entry.resize_u8_rgba)(
            ResizeInputU8Rgba {
                data: &[],
                width: 1,
                height: 1,
                row_stride_elements: 4
            },
            ResizeOutputU8Rgba {
                data: &mut output,
                width: 1,
                height: 1,
                row_stride_elements: 4
            },
            &ResizeParams::default(),
        )
        .is_err());
        assert_eq!(output, [91; 4]);
    }
}
