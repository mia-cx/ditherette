//! Untimed score recipes and exact evidence, including values that cannot be rendered.
use ditherette_bench::{
    paired::{native::*, *},
    verification::*,
};
use ditherette_bench_api::verification::*;
use ditherette_wasm::{
    bench_subjects::{self, scores, BenchSubject},
    spec,
};

#[path = "fixtures/paired_model.rs"]
mod model;

#[test]
fn paired_gate_rejects_a_bit_different_score_even_with_faster_fake_samples() {
    let (mut prepared, mut trials) = model::fixture();
    prepared.experiment.reference_state = ReferenceState::Frozen;
    let case = &mut prepared.experiment.cases[0];
    let metric = MetricFamily::Euclidean;
    let operation = NativeOperation::MetricScores { metric };
    case.identity = operation.identity(case.source, &case.rgba).unwrap();
    case.reference_subject = metric.reference_subject().into();
    case.accepted_subject = metric.prod_subject().into();
    case.candidate_subject = metric.prod_subject().into();
    case.measurement.scope = operation.scope();
    case.native = Some(operation);
    for trial in &mut trials {
        trial.measurement = case.measurement.clone();
        for record in [&mut trial.reference, &mut trial.output] {
            record.case = case.identity.clone();
            record.output.pixels = Pixels::Scores { values: vec![0.0] };
        }
        trial.reference.implementation.subject = case.reference_subject.clone();
        trial.output.implementation.subject = metric.prod_subject().into();
    }
    assert_eq!(compare(&prepared, &trials).gate, Gate::Pass);
    trials[1].output.output.pixels = Pixels::Scores { values: vec![-0.0] };
    assert_eq!(compare(&prepared, &trials).gate, Gate::Incorrect);
}

fn run(
    metric: MetricFamily,
    source: Dimensions,
    rgba: &[u8],
    production: bool,
) -> VerificationOutput {
    let operation = NativeOperation::MetricScores { metric };
    let request = operation.reference_request(source, rgba).unwrap();
    let registry = bench_subjects::bench_subjects();
    let id = if production {
        metric.prod_subject()
    } else {
        metric.reference_subject()
    };
    let BenchSubject::Conformance(subject) = registry
        .iter()
        .find(|s| s.descriptor().id.as_str() == id)
        .unwrap()
    else {
        panic!("typed score subject");
    };
    if production {
        assert_eq!(
            subject.descriptor.default_oracle.as_ref().unwrap().as_str(),
            metric.reference_subject()
        );
    }
    (subject.run)(&request).unwrap()
}

#[test]
fn all_seven_batches_match_frozen_score_bits_and_cyclic_pair_order() {
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
    for metric in MetricFamily::ALL {
        let expected = run(metric, source, &rgba, false);
        let actual = run(metric, source, &rgba, true);
        // Serialized score bits, not f32 PartialEq, are the exact output contract.
        assert_eq!(
            serde_json::to_value(&actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        let operation = NativeOperation::MetricScores { metric };
        let request = operation.reference_request(source, &rgba).unwrap();
        let pairs = scores::prepare_pairs(request.source(), metric).unwrap();
        let first = spec::color::rgb8_to_coordinates([rgba[0], rgba[1], rgba[2]], metric.space());
        let last = &rgba[rgba.len() - 4..];
        assert_eq!(
            pairs.last().unwrap(),
            &[
                spec::color::rgb8_to_coordinates([last[0], last[1], last[2]], metric.space()),
                first
            ]
        );
        let mut values = vec![0.0; pairs.len()];
        scores::score_into(&pairs, &mut values, metric.prod_function());
        let Pixels::Scores { values: reference } = expected.pixels else {
            panic!("scores")
        };
        assert!(values
            .iter()
            .zip(reference)
            .all(|(a, b)| a.to_bits() == b.to_bits()));
        assert!(render_rgba(&actual).unwrap().is_none());
        assert_eq!(operation.scope(), CallScope::NativeMetricScores);
    }
}

#[test]
fn score_fixture_identity_binds_all_source_bytes_metric_space_and_pair_recipe() {
    let source = Dimensions {
        width: 2,
        height: 1,
    };
    let rgba = [0, 0, 0, 0, 255, 255, 255, 255];
    let mut identities = std::collections::HashSet::new();
    for metric in MetricFamily::ALL {
        let operation = NativeOperation::MetricScores { metric };
        let identity = operation.identity(source, &rgba).unwrap();
        assert!(identities.insert(identity.settings.0));
        assert_eq!(
            identity.semantics.recipe,
            "metric-cyclic-successor-frozen-forward"
        );
        assert_eq!(identity.semantics.version, 1);
        for i in 0..rgba.len() {
            let mut changed = rgba;
            changed[i] ^= 1;
            assert_ne!(
                identity.input,
                operation.identity(source, &changed).unwrap().input
            );
        }
        assert!(operation.identity(source, &rgba[..4]).is_err());
        assert_eq!(
            serde_json::from_str::<NativeOperation>(&serde_json::to_string(&operation).unwrap())
                .unwrap(),
            operation
        );
    }
    for (metric, expected) in [
        (MetricFamily::Euclidean, 3.0f32),
        (MetricFamily::Compuphase, 8.996094),
        (MetricFamily::Rec601, 1.0),
        (MetricFamily::Rec709, 1.0),
    ] {
        let Pixels::Scores { values } = run(metric, source, &rgba, true).pixels else {
            panic!("scores")
        };
        assert!(
            values.iter().all(|v| v.to_bits() == expected.to_bits()),
            "{metric:?}: {values:?}"
        );
    }
}

#[test]
fn score_evidence_rejects_nonfinite_lengths_and_bit_differences_without_images() {
    let source = Dimensions {
        width: 2,
        height: 1,
    };
    let case = NativeOperation::MetricScores {
        metric: MetricFamily::Euclidean,
    }
    .identity(source, &[0; 8])
    .unwrap();
    let output = VerificationOutput {
        dimensions: source,
        pixels: Pixels::Scores {
            values: vec![0.0, 1.0],
        },
        warnings: vec![],
    };
    let record = |role: &str| RecordedOutput {
        case: case.clone(),
        implementation: ImplementationIdentity {
            subject: format!("{role}:metric:euclidean:cyclic-scores-v1"),
            artifact: ArtifactIdentity {
                revision: "a".repeat(40),
                content: content_digest(role.as_bytes()),
            },
        },
        output: output.clone(),
    };
    let mut triplet = ThreeWayOutputs {
        reference_state: ReferenceState::Frozen,
        reference: Some(record("spec")),
        accepted: Some(record("prod")),
        candidate: Some(record("candidate")),
    };
    assert!(verify_three_way(&case, &triplet, VerificationBounds::exact()).release_conformant());
    triplet.candidate.as_mut().unwrap().output.pixels = Pixels::Scores {
        values: vec![-0.0, 1.0],
    };
    let report = verify_three_way(&case, &triplet, VerificationBounds::exact());
    assert_eq!(report.status, VerificationStatus::Failed);
    let difference = report.reference_candidate.unwrap();
    assert_eq!(difference.scores.unwrap().differing_scores, 1);
    assert!(difference.rgba.is_none() && difference.coordinates.is_none());
    let serialized = serde_json::to_string(&triplet).unwrap();
    assert!(serialized.contains("score_bits"));
    let decoded: ThreeWayOutputs = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        verify_three_way(&case, &decoded, VerificationBounds::exact()).status,
        VerificationStatus::Failed
    );
    for invalid in [vec![0.0], vec![0.0, f32::NAN], vec![0.0, f32::INFINITY]] {
        triplet.candidate.as_mut().unwrap().output.pixels = Pixels::Scores { values: invalid };
        assert_eq!(
            verify_three_way(&case, &triplet, VerificationBounds::exact()).status,
            VerificationStatus::Incomplete
        );
    }
}
