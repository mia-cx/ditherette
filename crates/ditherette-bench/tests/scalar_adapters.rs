//! Untimed execution of the scalar plan through its real component adapters and frozen oracle.

use ditherette_bench::paired::{
    native::NativeOperation,
    scalar::{experiment, Comparison},
};
use ditherette_bench_api::verification::*;
use ditherette_wasm::{
    bench_subjects::{
        self, fields::PreparedComponent, reference::ReferenceRequest, scalar, scores,
        verification::indexed_output, BenchSubject,
    },
    image::{ImageDimensions, ImageView},
    spec::contract::request as spec,
};

#[test]
fn scalar_plan_covers_every_family_and_changes_only_the_accepted_role() {
    let spec = experiment(Comparison::SpecProd, "untimed fixture".into()).unwrap();
    let prod = experiment(Comparison::ProdProd, "untimed fixture".into()).unwrap();
    assert_eq!(spec.cases.len(), 101);
    for (prefix, count) in [
        ("resize-", 10),
        ("forward-", 7),
        ("inverse-", 7),
        ("reconstruct-", 7),
        ("source-construction-", 7),
        ("placement-", 7),
        ("scores-", 7),
        ("quantize-", 15),
        ("threshold-", 6),
        ("perturb-", 7),
        ("diffusion-", 8),
        ("yliluoma-", 4),
        ("tiny-", 9),
    ] {
        assert_eq!(
            spec.cases
                .iter()
                .filter(|case| case.name.starts_with(prefix))
                .count(),
            count,
            "{prefix}"
        );
    }
    for (a, b) in spec.cases.iter().zip(prod.cases) {
        assert_eq!(a.identity, b.identity);
        assert_eq!(a.measurement, b.measurement);
        assert_eq!(a.accepted_subject, a.reference_subject);
        assert_eq!(a.candidate_subject, b.candidate_subject);
        assert_eq!(b.accepted_subject, b.candidate_subject);
        assert!(a.browser.is_none());
    }
}

#[test]
fn frozen_timing_adapters_match_registered_oracles_without_running_timers() {
    let plan = experiment(Comparison::SpecProd, "untimed fixture".into()).unwrap();
    let registry = bench_subjects::bench_subjects();
    let source = Dimensions {
        width: 3,
        height: 2,
    };
    let rgba = [
        0, 127, 255, 255, 1, 2, 3, 0, 24, 190, 71, 128, 255, 0, 130, 254, 90, 33, 251, 65, 241,
        242, 243, 255,
    ];
    for case in plan.cases {
        let Some(operation) = case.native else {
            continue;
        };
        let request = operation.reference_request(source, &rgba).unwrap();
        let run = |id: &str| {
            let BenchSubject::Conformance(subject) = registry
                .iter()
                .find(|s| s.descriptor().id.as_str() == id)
                .unwrap()
            else {
                panic!("typed subject")
            };
            (subject.run)(&request).unwrap()
        };
        let expected = run(&case.reference_subject);
        let actual = match operation {
            NativeOperation::FieldComponent { component } => {
                let mut batch = PreparedComponent::new(
                    component,
                    spec::Source {
                        width: source.width,
                        height: source.height,
                        data: &rgba,
                    },
                )
                .unwrap();
                batch.run_selected(false);
                let frozen = batch.output();
                batch.run_selected(true);
                assert_eq!(
                    batch.output(),
                    run(&case.candidate_subject),
                    "production {}",
                    case.name
                );
                frozen
            }
            NativeOperation::PerturbComponent { .. } => {
                let mut batch = scalar::PerturbBatch::new(&request).unwrap();
                batch.run(false);
                let frozen = batch.output();
                batch.run(true);
                assert_eq!(
                    batch.output(),
                    run(&case.candidate_subject),
                    "production {}",
                    case.name
                );
                frozen
            }
            NativeOperation::ColorForward { space } => {
                let mut output = expected.clone();
                let Pixels::Color { coordinates, .. } = &mut output.pixels else {
                    panic!("color output")
                };
                coordinates.fill(f32::NAN);
                scalar::forward_into(
                    ImageView::packed(
                        &rgba,
                        ImageDimensions::new(source.width, source.height).unwrap(),
                    )
                    .unwrap(),
                    coordinates,
                    space,
                );
                output
            }
            NativeOperation::MetricScores { metric } => {
                let pairs = scores::prepare_pairs(request.source(), metric).unwrap();
                let mut values = vec![f32::NAN; pairs.len()];
                scores::score_into(&pairs, &mut values, metric.reference_function());
                VerificationOutput {
                    dimensions: source,
                    pixels: Pixels::Scores { values },
                    warnings: vec![],
                }
            }
            NativeOperation::Quantize { .. }
            | NativeOperation::Diffusion { .. }
            | NativeOperation::Yliluoma { .. } => {
                let ReferenceRequest::Processing(request) = request else {
                    panic!("indexed request")
                };
                indexed_output(&scalar::indexed_call(request).unwrap())
            }
            _ => panic!("unmatched scalar operation"),
        };
        assert_eq!(actual, expected, "frozen {}", case.name);
    }
}
