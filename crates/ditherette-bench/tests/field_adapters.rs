//! Small deterministic fixtures only. No collector or benchmark timing runs here.
use ditherette_bench::{
    paired::{fields::*, native::NativeOperation, quantize::QuantizeSettings, CallScope},
    verification::*,
};
use ditherette_bench_api::verification::*;
use ditherette_wasm::{
    bench_subjects::{
        self, field_calls,
        fields::{Component, PreparedComponent},
        BenchSubject,
    },
    spec,
};

const SPACES: [WorkingSpace; 7] = [
    WorkingSpace::Srgb,
    WorkingSpace::LinearRgb,
    WorkingSpace::Oklab,
    WorkingSpace::Oklch,
    WorkingSpace::Cielab,
    WorkingSpace::Cielch,
    WorkingSpace::Ycbcr,
];
const SOURCE: Dimensions = Dimensions {
    width: 3,
    height: 2,
};
const RGBA: [u8; 24] = [
    0, 0, 0, 0, 255, 255, 255, 127, 255, 0, 0, 128, 0, 255, 0, 255, 0, 0, 255, 1, 17, 33, 71, 254,
];

fn outputs(
    operation: &NativeOperation,
    production: &str,
) -> (VerificationOutput, VerificationOutput) {
    let registry = bench_subjects::bench_subjects();
    let request = operation.reference_request(SOURCE, &RGBA).unwrap();
    let run = |id: &str| {
        let BenchSubject::Conformance(subject) = registry
            .iter()
            .find(|s| s.descriptor().id.as_str() == id)
            .unwrap()
        else {
            panic!("conformance subject")
        };
        if id == production {
            assert_eq!(
                subject.descriptor.default_oracle.as_ref().unwrap().as_str(),
                operation.reference_subject()
            );
        }
        assert_eq!(subject.operation, request.semantics().operation);
        (subject.run)(&request).unwrap()
    };
    (run(operation.reference_subject()), run(production))
}

fn exact(expected: &VerificationOutput, actual: &VerificationOutput) {
    assert_eq!(
        serde_json::to_value(expected).unwrap(),
        serde_json::to_value(actual).unwrap()
    );
}

#[test]
fn original_inverse_exports_and_source_conversion_batches_match_frozen_bits_and_alpha() {
    for space in SPACES {
        for component in [
            Component::Inverse { space },
            Component::SourceConversion { space },
        ] {
            let operation = NativeOperation::FieldComponent { component };
            let (expected, actual) = outputs(&operation, component.prod_subject());
            exact(&expected, &actual);
            let mut prepared = PreparedComponent::new(
                component,
                spec::contract::request::Source {
                    width: SOURCE.width,
                    height: SOURCE.height,
                    data: &RGBA,
                },
            )
            .unwrap();
            prepared.run_production();
            exact(&expected, &prepared.output());
            match actual.pixels {
                Pixels::Rgba8 { data } => assert_eq!(
                    data.chunks_exact(4).map(|p| p[3]).collect::<Vec<_>>(),
                    vec![0, 127, 128, 255, 1, 254]
                ),
                Pixels::Color { alpha, .. } => assert_eq!(alpha, vec![0, 127, 128, 255, 1, 254]),
                _ => panic!("typed color boundary"),
            }
        }
    }
}

#[test]
fn field_grids_and_adaptive_masks_use_exact_scalar_bits_without_fake_rendering() {
    let components = [
        Component::Field {
            field: Field::Bayer {
                size: BayerSize::Two,
            },
        },
        Component::Field {
            field: Field::Bayer {
                size: BayerSize::Four,
            },
        },
        Component::Field {
            field: Field::Bayer {
                size: BayerSize::Eight,
            },
        },
        Component::Field {
            field: Field::Bayer {
                size: BayerSize::Sixteen,
            },
        },
        Component::Field {
            field: Field::Random { seed: 0x12345678 },
        },
        Component::Placement {
            space: WorkingSpace::Oklab,
            placement: Placement::Adaptive {
                radius: 1,
                threshold: 10.0,
                softness: 5.0,
            },
        },
        Component::Placement {
            space: WorkingSpace::Oklch,
            placement: Placement::Adaptive {
                radius: 2,
                threshold: 10.0,
                softness: 5.0,
            },
        },
    ];
    let mut settings = std::collections::HashSet::new();
    for component in components {
        let operation = NativeOperation::FieldComponent { component };
        let identity = operation.identity(SOURCE, &RGBA).unwrap();
        assert!(settings.insert(identity.settings.0));
        if matches!(component, Component::Field { .. }) {
            assert_eq!(identity.semantics.space, None);
        }
        let (expected, actual) = outputs(&operation, component.prod_subject());
        exact(&expected, &actual);
        assert!(matches!(actual.pixels, Pixels::Scores { .. }));
        assert!(render_rgba(&actual).unwrap().is_none());
        let request = operation.reference_request(SOURCE, &RGBA).unwrap();
        let mut batch = PreparedComponent::new(component, request.source()).unwrap();
        batch.run_production();
        exact(&expected, &batch.output());
        assert_eq!(
            serde_json::from_str::<NativeOperation>(&serde_json::to_string(&operation).unwrap())
                .unwrap(),
            operation
        );
    }
}

#[test]
fn complete_processor_calls_preserve_byte_boundary_palette_warnings_and_cross_space_composition() {
    use ditherette_bench::paired::quantize::{AlphaPolicy, MatchPolicy, PaletteEntry};
    for space in SPACES {
        let perturb = PerturbPolicy {
            space,
            strength: 0.7,
            field: Field::Random { seed: 123 },
            placement: Placement::Adaptive {
                radius: 1,
                threshold: 10.0,
                softness: 5.0,
            },
        };
        let operation = NativeOperation::Perturb { settings: perturb };
        let (expected, actual) = outputs(&operation, field_calls::PERTURB_SUBJECT);
        exact(&expected, &actual);
        assert_eq!(operation.scope(), CallScope::NativeCompleteCall);
        for matching in [MatchPolicy::SrgbCompuphase, MatchPolicy::OklchHueArc] {
            let settings = SeparableSettings {
                perturb,
                quantize: QuantizeSettings {
                    palette: vec![
                        PaletteEntry::Color { rgb: [0; 3] },
                        PaletteEntry::Color { rgb: [255; 3] },
                        PaletteEntry::Transparent {},
                    ],
                    alpha: AlphaPolicy::Preserve { threshold: 0.5 },
                    matching,
                },
            };
            let operation = NativeOperation::Separable { settings };
            let (expected, actual) = outputs(&operation, field_calls::SEPARABLE_SUBJECT);
            exact(&expected, &actual);
            let request = operation.reference_request(SOURCE, &RGBA).unwrap();
            let call = field_calls::CompleteCall::new(&request).unwrap();
            let mut processor = field_calls::processor().unwrap();
            call.run(&mut processor).unwrap();
            exact(&actual, &call.output(&mut processor).unwrap());
            assert_eq!(
                operation
                    .identity(SOURCE, &RGBA)
                    .unwrap()
                    .semantics
                    .operation,
                Operation::DitherAndQuantize
            );
        }
    }
}

#[test]
fn identities_reject_missing_inputs_unsupported_fields_and_bind_every_policy_setting() {
    let settings = PerturbPolicy {
        space: WorkingSpace::Srgb,
        strength: 0.7,
        field: Field::Random { seed: 1 },
        placement: Placement::Adaptive {
            radius: 1,
            threshold: 10.0,
            softness: 5.0,
        },
    };
    let identity = NativeOperation::Perturb { settings }
        .identity(SOURCE, &RGBA)
        .unwrap();
    for changed in [
        PerturbPolicy {
            space: WorkingSpace::Oklab,
            ..settings
        },
        PerturbPolicy {
            strength: 0.6,
            ..settings
        },
        PerturbPolicy {
            field: Field::Random { seed: 2 },
            ..settings
        },
        PerturbPolicy {
            placement: Placement::Adaptive {
                radius: 2,
                threshold: 10.0,
                softness: 5.0,
            },
            ..settings
        },
        PerturbPolicy {
            placement: Placement::Adaptive {
                radius: 1,
                threshold: 11.0,
                softness: 5.0,
            },
            ..settings
        },
        PerturbPolicy {
            placement: Placement::Adaptive {
                radius: 1,
                threshold: 10.0,
                softness: 6.0,
            },
            ..settings
        },
    ] {
        assert_ne!(
            identity.settings,
            NativeOperation::Perturb { settings: changed }
                .identity(SOURCE, &RGBA)
                .unwrap()
                .settings
        );
    }
    assert!(NativeOperation::Perturb { settings }
        .identity(SOURCE, &RGBA[..4])
        .is_err());
    assert!(NativeOperation::Perturb {
        settings: PerturbPolicy {
            field: Field::BlueNoise {},
            ..settings
        }
    }
    .identity(SOURCE, &RGBA)
    .is_err());
    assert!(NativeOperation::FieldComponent {
        component: Component::Field {
            field: Field::BlueNoise {}
        }
    }
    .identity(SOURCE, &RGBA)
    .is_err());
}
