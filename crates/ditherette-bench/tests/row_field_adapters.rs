//! Resolve S36 recipes through existing full-call adapters, without collecting timings.
#[path = "../examples/support/row_fields.rs"]
mod row_fields;

use ditherette_bench::paired::{native::NativeOperation, CallScope};
use ditherette_bench_api::verification::Dimensions;
use ditherette_wasm::bench_subjects::{self, field_calls, quantize, BenchSubject};

#[test]
fn six_complete_recipes_bind_public_identity_and_exact_frozen_output() {
    let source = Dimensions {
        width: 9,
        height: 7,
    };
    let rgba: Vec<_> = (0..source.width * source.height)
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
    let registry = bench_subjects::bench_subjects();
    let recipes = row_fields::recipes();
    assert_eq!(recipes.len(), 6);
    let mut identities = std::collections::HashSet::new();
    for (name, native) in recipes {
        let identity = native.identity(source, &rgba).unwrap();
        assert!(identities.insert(identity.settings.0));
        assert_eq!(
            identity,
            row_fields::public(&native)
                .identity(source, &rgba, source)
                .unwrap()
        );
        assert_eq!(native.scope(), CallScope::NativeCompleteCall);
        let request = native.reference_request(source, &rgba).unwrap();
        let subject = match native {
            NativeOperation::Quantize { .. } => quantize::QUANTIZE_SUBJECT,
            NativeOperation::Perturb { .. } => field_calls::PERTURB_SUBJECT,
            NativeOperation::Separable { .. } => field_calls::SEPARABLE_SUBJECT,
            _ => unreachable!("S36 recipe"),
        };
        let run = |id: &str| {
            let BenchSubject::Conformance(entry) = registry
                .iter()
                .find(|entry| entry.descriptor().id.as_str() == id)
                .unwrap()
            else {
                panic!("typed full-call subject")
            };
            (entry.run)(&request).unwrap()
        };
        assert_eq!(run(subject), run(native.reference_subject()), "{name}");
    }
    assert_eq!(rgba, original);
}
