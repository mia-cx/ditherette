//! Exercise actual adapters and the full frozen verifier, without collecting samples.
#[path = "../examples/support/blue_noise.rs"]
mod blue_noise;
use ditherette_bench::{
    paired::{fields::Field, native::NativeOperation},
    verification::*,
};
use ditherette_bench_api::verification::*;
use ditherette_wasm::bench_subjects::{
    self, field_calls,
    fields::{Component, PreparedComponent},
    BenchSubject,
};

#[test]
fn blue_noise_thresholds_and_complete_calls_match_frozen_outputs_across_tile_edges() {
    let (source, rgba) = blue_noise::fixture();
    let component = Component::Field {
        field: Field::BlueNoise {},
    };
    let mut recipes = blue_noise::recipes();
    recipes.insert(
        0,
        (
            "thresholds".into(),
            NativeOperation::FieldComponent { component },
        ),
    );
    let registry = bench_subjects::bench_subjects();
    for (name, native) in recipes {
        let identity = native.identity(source, &rgba).unwrap();
        let request = native.reference_request(source, &rgba).unwrap();
        let production = match native {
            NativeOperation::FieldComponent { component } => component.prod_subject(),
            NativeOperation::Perturb { .. } => field_calls::PERTURB_SUBJECT,
            NativeOperation::Separable { .. } => field_calls::SEPARABLE_SUBJECT,
            _ => unreachable!(),
        };
        let run = |id| {
            let BenchSubject::Conformance(subject) = registry
                .iter()
                .find(|subject| subject.descriptor().id.as_str() == id)
                .unwrap()
            else {
                panic!("typed conformance adapter")
            };
            (subject.run)(&request).unwrap()
        };
        let expected = run(native.reference_subject());
        let actual = run(production);
        let record = |subject: &str, output: VerificationOutput| RecordedOutput {
            case: identity.clone(),
            output,
            implementation: ImplementationIdentity {
                subject: subject.into(),
                artifact: ArtifactIdentity {
                    revision: "a".repeat(40),
                    content: content_digest(subject.as_bytes()),
                },
            },
        };
        let outputs = ThreeWayOutputs {
            reference_state: ReferenceState::Frozen,
            reference: Some(record(native.reference_subject(), expected)),
            accepted: Some(record(production, actual.clone())),
            candidate: Some(record(production, actual.clone())),
        };
        let report = verify_three_way(&identity, &outputs, VerificationBounds::exact());
        assert!(report.release_conformant(), "{name}: {:?}", report.issues);
        if let Pixels::Scores { values } = &actual.pixels {
            assert_eq!(identity.semantics.space, None);
            let mut batch = PreparedComponent::new(component, request.source()).unwrap();
            batch.run_production();
            assert_eq!(
                serde_json::to_value(batch.output()).unwrap(),
                serde_json::to_value(&actual).unwrap()
            );
            for y in [0, 31, 32] {
                for x in [0, 31, 32, 63, 64] {
                    assert_eq!(
                        values[(y * source.width + x) as usize].to_bits(),
                        ditherette_wasm::spec::dither::blue_noise::blue_noise_at(x, y).to_bits()
                    );
                }
            }
        } else {
            assert_eq!(
                identity,
                blue_noise::public(&native)
                    .identity(source, &rgba, source)
                    .unwrap()
            );
            if let Pixels::Rgba8 { data } = &actual.pixels {
                assert!(data
                    .chunks_exact(4)
                    .zip(rgba.chunks_exact(4))
                    .all(|(a, b)| a[3] == b[3]));
            }
            let call = field_calls::CompleteCall::new(&request).unwrap();
            let mut processor = field_calls::processor().unwrap();
            call.run(&mut processor).unwrap();
            assert_eq!(
                serde_json::to_value(call.output(&mut processor).unwrap()).unwrap(),
                serde_json::to_value(&actual).unwrap()
            );
        }
    }
}
