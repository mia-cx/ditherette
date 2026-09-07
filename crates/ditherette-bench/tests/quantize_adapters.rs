//! Real operations without a timer. Timing workers use these same callable adapters.
use ditherette_bench::paired::{
    native::{NativeOperation, WorkingSpace},
    quantize::*,
};
use ditherette_bench_api::verification::*;
use ditherette_wasm::bench_subjects::{self, quantize as adapters, BenchSubject};

fn compare(operation: NativeOperation, subject: &str) -> VerificationOutput {
    let source = Dimensions {
        width: 17,
        height: 7,
    };
    let rgba: Vec<_> = (0..119u32)
        .flat_map(|i| {
            [
                (i * 73) as u8,
                (i * 31 + 19) as u8,
                (i * 17 + 113) as u8,
                (i * 43) as u8,
            ]
        })
        .collect();
    let original = rgba.clone();
    let request = operation.reference_request(source, &rgba).unwrap();
    let registry = bench_subjects::bench_subjects();
    let run = |id: &str| {
        let BenchSubject::Conformance(entry) = registry
            .iter()
            .find(|s| s.descriptor().id.as_str() == id)
            .unwrap()
        else {
            panic!("typed registry subject")
        };
        assert_eq!(entry.operation, request.semantics().operation);
        (entry.run)(&request).unwrap()
    };
    let expected = run(operation.reference_subject());
    let actual = run(subject);
    assert_eq!(actual, expected, "{subject}");
    assert_eq!(rgba, original);
    if let NativeOperation::Quantize { .. } = operation {
        let function = adapters::quantize_function(subject).unwrap();
        let actual_call = function(adapters::quantize_request(&request).unwrap()).unwrap();
        assert_eq!(
            bench_subjects::verification::indexed_output(&actual_call),
            actual
        );
    }
    actual
}

#[test]
fn full_native_quantize_call_matches_frozen_indices_palette_alpha_and_warnings() {
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::YcbcrEuclidean,
    ] {
        for count in [16, 64, 256] {
            for alpha in [
                AlphaPolicy::Preserve { threshold: 0.5 },
                AlphaPolicy::Premultiplied {},
                AlphaPolicy::Matte { rgb: [17, 33, 71] },
            ] {
                let mut palette: Vec<_> = (0..count - 1)
                    .map(|i| PaletteEntry::Color {
                        rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
                    })
                    .collect();
                palette.push(PaletteEntry::Transparent {});
                compare(
                    NativeOperation::Quantize {
                        settings: QuantizeSettings {
                            palette,
                            alpha,
                            matching,
                        },
                    },
                    adapters::QUANTIZE_SUBJECT,
                );
            }
        }
    }
    let output = compare(
        NativeOperation::Quantize {
            settings: QuantizeSettings {
                palette: vec![PaletteEntry::Transparent {}],
                alpha: AlphaPolicy::Preserve { threshold: 1.0 },
                matching: MatchPolicy::SrgbEuclidean,
            },
        },
        adapters::QUANTIZE_SUBJECT,
    );
    assert!(!output.warnings.is_empty());
    assert!(adapters::quantize_function("spec:quantize:request:v1").is_none());
}

#[test]
fn packed_forward_controls_keep_exact_coordinates_and_byte_alpha() {
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Cielab,
        WorkingSpace::Ycbcr,
    ] {
        let output = compare(
            NativeOperation::ColorForward { space },
            adapters::color_subject(space).unwrap(),
        );
        let Pixels::Color {
            coordinates,
            alpha,
            rendered_rgba,
            ..
        } = output.pixels
        else {
            panic!("packed coordinates")
        };
        assert_eq!(coordinates.len(), 119 * 3);
        assert_eq!(alpha.len(), 119);
        assert_eq!(rendered_rgba.unwrap().len(), 119 * 4);
    }
}
