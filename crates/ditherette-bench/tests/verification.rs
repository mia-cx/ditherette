use ditherette_bench::verification::*;
use ditherette_bench_api::verification::*;
use std::{collections::HashMap, fs};

fn case(operation: Operation) -> CaseIdentity {
    CaseIdentity {
        semantics: SemanticIdentity {
            operation,
            recipe: "fixture-v1".into(),
            version: 1,
            space: (operation != Operation::Resize).then_some(ColorSpace::Srgb),
        },
        input: content_digest(b"source"),
        settings: content_digest(b"normalized settings"),
        output: Dimensions {
            width: 2,
            height: 1,
        },
    }
}

fn indexed() -> VerificationOutput {
    VerificationOutput {
        dimensions: Dimensions {
            width: 2,
            height: 1,
        },
        warnings: vec![],
        pixels: Pixels::Indexed8 {
            indices: vec![0, 0],
            palette_rgba: vec![0, 0, 0, 255, 3, 4, 0, 255, 0, 0, 0, 0],
            transparent_index: Some(2),
        },
    }
}

fn outputs(case: &CaseIdentity, output: VerificationOutput) -> ThreeWayOutputs {
    let record = |role: &str| RecordedOutput {
        case: case.clone(),
        implementation: ImplementationIdentity {
            subject: format!("{role}:fixture:identity:scalar"),
            artifact: ArtifactIdentity {
                revision: "a".repeat(40),
                content: content_digest(role.as_bytes()),
            },
        },
        output: output.clone(),
    };
    ThreeWayOutputs {
        reference_state: ReferenceState::PreFreeze,
        reference: Some(record("spec")),
        accepted: Some(record("accepted")),
        candidate: Some(record("candidate")),
    }
}

#[test]
fn every_operation_requires_the_correct_output_and_a_frozen_reference_for_release() {
    for operation in [
        Operation::Resize,
        Operation::Perturb,
        Operation::Quantize,
        Operation::DitherAndQuantize,
        Operation::Process,
    ] {
        let case = case(operation);
        let mut output = indexed();
        if matches!(operation, Operation::Resize | Operation::Perturb) {
            output.pixels = Pixels::Rgba8 {
                data: vec![0, 0, 0, 255, 0, 0, 0, 255],
            };
        }
        let mut records = outputs(&case, output);
        let report = verify_three_way(&case, &records, VerificationBounds::exact());
        assert_eq!(report.status, VerificationStatus::Exact);
        assert!(!report.release_conformant());
        records.reference_state = ReferenceState::Frozen;
        assert!(
            verify_three_way(&case, &records, VerificationBounds::exact()).release_conformant()
        );
    }
}

#[test]
fn indexed_error_reuses_rgba_distances_without_numeric_auto_approval() {
    let case = case(Operation::Quantize);
    let mut records = outputs(&case, indexed());
    let Pixels::Indexed8 { indices, .. } = &mut records.candidate.as_mut().unwrap().output.pixels
    else {
        unreachable!()
    };
    indices[0] = 1;
    let report = verify_three_way(
        &case,
        &records,
        VerificationBounds {
            max_color_distance: 5.0,
            max_mean_color_distance: 2.5,
            max_rms_color_distance: 4.0,
        },
    );
    assert_eq!(report.status, VerificationStatus::NeedsVisualApproval);
    assert!(!report.release_conformant());
    let comparison = report.reference_candidate.unwrap();
    assert_eq!(comparison.differing_indices, Some(1));
    let rgba = comparison.rgba.unwrap();
    assert_eq!(rgba.max_color_distance, 5.0);
    assert_eq!(rgba.mean_color_distance, 2.5);
    assert_eq!(rgba.rms_color_distance, (12.5_f64).sqrt());
    assert_eq!(
        verify_three_way(&case, &records, VerificationBounds::exact()).status,
        VerificationStatus::Failed
    );
}

#[test]
fn palette_transparency_and_warning_metadata_cannot_hide_behind_equal_rendering() {
    let case = case(Operation::Quantize);
    for mismatch in ["palette", "transparency", "warning"] {
        let mut records = outputs(&case, indexed());
        let candidate = &mut records.candidate.as_mut().unwrap().output;
        let Pixels::Indexed8 {
            indices,
            palette_rgba,
            transparent_index,
        } = &mut candidate.pixels
        else {
            unreachable!()
        };
        match mismatch {
            "palette" => {
                palette_rgba.swap(0, 4);
                palette_rgba.swap(1, 5);
                indices.fill(1);
            }
            "transparency" => *transparent_index = None,
            "warning" => candidate.warnings.push(Warning {
                code: WarningCode::PaletteTruncated,
                message: "Palette truncated.".into(),
            }),
            _ => unreachable!(),
        }
        let report = verify_three_way(&case, &records, VerificationBounds::exact());
        assert_eq!(report.status, VerificationStatus::Failed, "{mismatch}");
        let comparison = report.reference_candidate.unwrap();
        assert!(!comparison.metadata_mismatches.is_empty());
        assert_eq!(comparison.rgba.unwrap().differing_bytes, 0);
    }
}

#[test]
fn missing_or_unrelated_required_evidence_never_passes() {
    let case = case(Operation::Quantize);
    for role in ["reference", "accepted", "candidate"] {
        let mut records = outputs(&case, indexed());
        match role {
            "reference" => records.reference = None,
            "accepted" => records.accepted = None,
            _ => records.candidate = None,
        }
        assert_eq!(
            verify_three_way(&case, &records, VerificationBounds::exact()).status,
            VerificationStatus::Incomplete
        );
    }
    for mismatch in ["input", "settings", "operation", "dimensions", "revision"] {
        let mut records = outputs(&case, indexed());
        let candidate = records.candidate.as_mut().unwrap();
        match mismatch {
            "input" => candidate.case.input = content_digest(b"other input"),
            "settings" => candidate.case.settings = content_digest(b"other settings"),
            "operation" => candidate.case.semantics.operation = Operation::Process,
            "dimensions" => candidate.output.dimensions.width = 1,
            "revision" => candidate.implementation.artifact.revision = "short-sha".into(),
            _ => unreachable!(),
        }
        assert_eq!(
            verify_three_way(&case, &records, VerificationBounds::exact()).status,
            VerificationStatus::Incomplete,
            "{mismatch}"
        );
    }
    let mut records = outputs(&case, indexed());
    let Pixels::Indexed8 { indices, .. } = &mut records.accepted.as_mut().unwrap().output.pixels
    else {
        unreachable!()
    };
    indices[0] = 1;
    assert_eq!(
        verify_three_way(&case, &records, VerificationBounds::exact()).status,
        VerificationStatus::Failed
    );
}

#[test]
fn packed_coordinates_use_numeric_error_and_preserve_nonfinite_raw_bits() {
    let mut case = case(Operation::Color);
    case.semantics.space = Some(ColorSpace::Oklab);
    let output = VerificationOutput {
        dimensions: case.output,
        warnings: vec![],
        pixels: Pixels::Color {
            space: ColorSpace::Oklab,
            coordinates: vec![0.0; 6],
            alpha: vec![255; 2],
            rendered_rgba: None,
        },
    };
    let mut records = outputs(&case, output);
    let Pixels::Color { coordinates, .. } = &mut records.candidate.as_mut().unwrap().output.pixels
    else {
        unreachable!()
    };
    coordinates[0] = 0.5;
    let report = verify_three_way(&case, &records, VerificationBounds::bounded_default());
    let comparison = report.reference_candidate.unwrap();
    assert!(comparison.rgba.is_none());
    let numeric = comparison.coordinates.unwrap();
    assert_eq!(numeric.differing_coordinates, 1);
    assert_eq!(numeric.max_abs_delta, 0.5);
    assert_eq!(numeric.mean_abs_delta, 0.5 / 6.0);
    assert_eq!(numeric.rms_delta, (0.25_f64 / 6.0).sqrt());
    assert_eq!(report.status, VerificationStatus::Failed);

    let Pixels::Color { coordinates, .. } = &mut records.candidate.as_mut().unwrap().output.pixels
    else {
        unreachable!()
    };
    coordinates[0] = f32::from_bits(0x7fc00001);
    let decoded: ThreeWayOutputs =
        serde_json::from_slice(&serde_json::to_vec(&records).unwrap()).unwrap();
    let Pixels::Color { coordinates, .. } = &decoded.candidate.as_ref().unwrap().output.pixels
    else {
        unreachable!()
    };
    assert_eq!(coordinates[0].to_bits(), 0x7fc00001);
    assert_eq!(
        verify_three_way(&case, &decoded, VerificationBounds::exact()).status,
        VerificationStatus::Incomplete
    );
}

#[test]
fn failures_keep_raw_outputs_and_all_review_images_without_overwriting() {
    let case = case(Operation::Process);
    let mut records = outputs(&case, indexed());
    let Pixels::Indexed8 { indices, .. } = &mut records.candidate.as_mut().unwrap().output.pixels
    else {
        unreachable!()
    };
    indices[0] = 1;
    let directory =
        std::env::temp_dir().join(format!("ditherette-review-fixture-{}", std::process::id()));
    let report =
        verify_and_preserve(&case, &records, VerificationBounds::exact(), &directory).unwrap();
    assert_eq!(report.status, VerificationStatus::Failed);
    let raw = fs::read_to_string(directory.join("results.json")).unwrap();
    assert!(raw.contains(&"a".repeat(40)));
    for name in [
        "reference",
        "accepted",
        "candidate",
        "difference-reference-candidate",
        "difference-reference-accepted",
        "difference-accepted-candidate",
    ] {
        let image = image::open(directory.join(format!("{name}.png"))).unwrap();
        assert_eq!((image.width(), image.height()), (2, 1));
    }
    let difference = image::open(directory.join("difference-reference-candidate.png"))
        .unwrap()
        .to_rgba8();
    assert_eq!(difference.get_pixel(0, 0).0, [3, 4, 0, 255]);
    assert!(write_review_artifacts(&directory, &case, &records, &report).is_err());
    assert_eq!(
        fs::read_to_string(directory.join("results.json")).unwrap(),
        raw
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn identities_use_full_sha256_and_canonical_typed_settings() {
    let digest = content_digest(b"abc")
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        digest,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let a = HashMap::from([("seed", 3), ("strength", 1)]);
    let b = HashMap::from([("strength", 1), ("seed", 3)]);
    assert_eq!(settings_digest(&a).unwrap(), settings_digest(&b).unwrap());
    assert_ne!(
        input_digest(
            Dimensions {
                width: 1,
                height: 2
            },
            &[0; 8]
        ),
        input_digest(
            Dimensions {
                width: 2,
                height: 1
            },
            &[0; 8]
        )
    );
    assert_ne!(
        input_digest(
            Dimensions {
                width: 2,
                height: 1
            },
            &[0; 8]
        ),
        input_digest(
            Dimensions {
                width: 2,
                height: 1
            },
            &[1; 8]
        )
    );
}

#[test]
fn malformed_storage_cannot_pass_the_shared_rgba_engine() {
    assert!(!verify_with_bounds(&[1, 2, 3], &[1, 2, 3], VerificationBounds::exact()).passed);
    let case = case(Operation::Quantize);
    let mut records = outputs(&case, indexed());
    let Pixels::Indexed8 { indices, .. } = &mut records.candidate.as_mut().unwrap().output.pixels
    else {
        unreachable!()
    };
    indices[0] = 255;
    assert_eq!(
        verify_three_way(&case, &records, VerificationBounds::exact()).status,
        VerificationStatus::Incomplete
    );
}
