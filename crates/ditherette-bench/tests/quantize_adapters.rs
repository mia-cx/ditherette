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
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
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
        WorkingSpace::Oklch,
        WorkingSpace::Cielch,
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

#[test]
fn executable_inventory_resolves_every_matching_component_and_frozen_oracle() {
    let registry = bench_subjects::bench_subjects();
    let mut entries = vec![(adapters::QUANTIZE_SUBJECT, "spec:quantize:request:v1")];
    for (space, oracle) in [
        (WorkingSpace::Srgb, "spec:color:srgb:f32-roundtrip-v1"),
        (
            WorkingSpace::LinearRgb,
            "spec:color:linear-rgb:f32-roundtrip-v1",
        ),
        (WorkingSpace::Oklab, "spec:color:oklab:f32-roundtrip-v1"),
        (WorkingSpace::Oklch, "spec:color:oklch:f32-roundtrip-v1"),
        (WorkingSpace::Cielab, "spec:color:cielab:f32-roundtrip-v1"),
        (WorkingSpace::Cielch, "spec:color:cielch:f32-roundtrip-v1"),
        (WorkingSpace::Ycbcr, "spec:color:ycbcr:f32-roundtrip-v1"),
    ] {
        entries.push((adapters::color_subject(space).unwrap(), oracle));
    }
    for metric in bench_subjects::scores::MetricFamily::ALL {
        entries.push((metric.prod_subject(), metric.reference_subject()));
    }
    for (id, oracle) in entries {
        let subject = registry
            .iter()
            .find(|s| s.descriptor().id.as_str() == id)
            .unwrap();
        assert_eq!(
            subject
                .descriptor()
                .default_oracle
                .as_ref()
                .unwrap()
                .as_str(),
            oracle
        );
        let expected = registry
            .iter()
            .find(|s| s.descriptor().id.as_str() == oracle)
            .unwrap();
        assert_eq!(
            subject.descriptor().capabilities.pixel_formats,
            expected.descriptor().capabilities.pixel_formats
        );
        for entry in [subject, expected] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(&entry.descriptor().source_file);
            assert!(path.is_file(), "{}", path.display());
        }
    }
}
