#![cfg(feature = "bench-subjects")]

use ditherette_bench_api::{ResizeInputU8Rgba, ResizeOutputU8Rgba, ResizeParams};
use ditherette_wasm::bench_subjects::{bench_subjects, BenchSubject};

#[test]
fn prepared_subject_matches_frozen_on_fractional_mips() {
    let registry = bench_subjects();
    let entry = |id: &str| {
        registry
            .iter()
            .find_map(|subject| match subject {
                BenchSubject::Resize(resize) if resize.descriptor.id.to_string() == id => {
                    Some(resize)
                }
                _ => None,
            })
            .unwrap()
    };
    let reference = entry("spec:resize:trilinear:mip-area");
    let production = entry("prod:resize:trilinear:mip-area");
    assert_eq!(
        production
            .descriptor
            .default_oracle
            .as_ref()
            .unwrap()
            .to_string(),
        reference.descriptor.id.to_string()
    );
    let source: Vec<u8> = (0..129 * 97 * 4).map(|x| (x * 71 + 19) as u8).collect();
    let mut expected = vec![0; 13 * 11 * 4];
    let mut actual = expected.clone();
    for (subject, output) in [(reference, &mut expected), (production, &mut actual)] {
        (subject.resize_u8_rgba)(
            ResizeInputU8Rgba {
                data: &source,
                width: 129,
                height: 97,
                row_stride_elements: 129 * 4,
            },
            ResizeOutputU8Rgba {
                data: output,
                width: 13,
                height: 11,
                row_stride_elements: 13 * 4,
            },
            &ResizeParams::default(),
        )
        .unwrap();
    }
    assert_eq!(actual, expected);
}
