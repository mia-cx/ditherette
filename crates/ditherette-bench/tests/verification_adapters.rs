use ditherette_bench::verification::{
    content_digest, input_digest, settings_digest, verify_three_way, VerificationBounds,
    VerificationStatus,
};
use ditherette_bench_api::verification::*;
use ditherette_wasm::{
    bench_subjects::verification::{indexed_output, production_resize, reference_resize},
    image::{
        contracts::{
            IndexedImage, NormalizedPalette, ProcessWarning, WarningCode as PublicWarning,
        },
        ImageBuf, ImageDimensions, PaletteIndex8, RowStride,
    },
    spec::contract::request::{Anchor, Output, ResizePolicy, ResizeRequest, Source},
};

#[test]
fn concrete_resize_requests_use_the_existing_reference_and_production_registry() {
    let source = [10, 20, 30, 255, 40, 50, 60, 0];
    let request = ResizeRequest {
        version: 1,
        source: Source {
            width: 2,
            height: 1,
            data: &source,
        },
        output: Output {
            width: 2,
            height: 1,
            resize: ResizePolicy::Nearest {
                anchor: Anchor::Center,
            },
        },
    };
    let case = VerificationCase {
        identity: CaseIdentity {
            semantics: SemanticIdentity {
                operation: Operation::Resize,
                recipe: "nearest".into(),
                version: 1,
                space: None,
            },
            input: input_digest(
                Dimensions {
                    width: 2,
                    height: 1,
                },
                &source,
            ),
            settings: settings_digest(&request.output).unwrap(),
            output: Dimensions {
                width: 2,
                height: 1,
            },
        },
        request,
    };
    let subject = |role: &str, run| VerificationSubject {
        identity: ImplementationIdentity {
            subject: format!("{role}:resize:nearest:scalar"),
            artifact: ArtifactIdentity {
                revision: "b".repeat(40),
                content: content_digest(role.as_bytes()),
            },
        },
        semantics: case.identity.semantics.clone(),
        run,
    };
    let reference = subject("spec", reference_resize).evaluate(&case).unwrap();
    let accepted = subject("accepted", production_resize)
        .evaluate(&case)
        .unwrap();
    let candidate = subject("candidate", production_resize)
        .evaluate(&case)
        .unwrap();
    assert_eq!(
        reference.output.pixels,
        Pixels::Rgba8 {
            data: source.to_vec()
        }
    );
    let outputs = ThreeWayOutputs {
        reference_state: ReferenceState::PreFreeze,
        reference: Some(reference),
        accepted: Some(accepted),
        candidate: Some(candidate),
    };
    assert_eq!(
        verify_three_way(&case.identity, &outputs, VerificationBounds::exact()).status,
        VerificationStatus::Exact
    );
    assert_eq!(source, [10, 20, 30, 255, 40, 50, 60, 0]);

    let identity = subject("spec", reference_resize);
    let mut wrong_case = case;
    wrong_case.identity.semantics.version = 2;
    assert!(identity.evaluate(&wrong_case).is_err());
}

#[test]
fn storage_adapter_retains_palette_duplicates_warning_text_and_logical_rows() {
    let dimensions = ImageDimensions::new(1, 2).unwrap();
    let image = IndexedImage {
        indices: ImageBuf::<PaletteIndex8>::from_vec_strided(
            vec![1, 99, 0],
            dimensions,
            RowStride::new(2).unwrap(),
        )
        .unwrap(),
        palette: NormalizedPalette {
            rgba: vec![0, 0, 0, 0, 0, 0, 0, 0],
            transparent_index: Some(0),
        },
        warnings: vec![ProcessWarning {
            code: PublicWarning::TransparentOnly,
            message: "Only transparent colors were supplied.".into(),
        }],
    };
    let output = indexed_output(&image);
    assert_eq!(
        output.pixels,
        Pixels::Indexed8 {
            indices: vec![1, 0],
            palette_rgba: image.palette.rgba.clone(),
            transparent_index: Some(0)
        }
    );
    assert_eq!(
        output.warnings,
        vec![Warning {
            code: WarningCode::TransparentOnly,
            message: image.warnings[0].message.clone()
        }]
    );
}

#[test]
fn joined_trilinear_adapter_matches_frozen_odd_mips_and_anchor_bytes() {
    let mut request = ResizeRequest {
        version: 1,
        source: Source {
            width: 3,
            height: 1,
            data: &[0, 0, 0, 255, 0, 0, 0, 255, 255, 0, 0, 255],
        },
        output: Output {
            width: 1,
            height: 1,
            resize: ResizePolicy::Trilinear {
                anchor: Anchor::Center,
            },
        },
    };
    for (anchor, red) in [
        (Anchor::Left, 68),
        (Anchor::Center, 85),
        (Anchor::Right, 103),
    ] {
        request.output.resize = ResizePolicy::Trilinear { anchor };
        let expected = reference_resize(&request).unwrap();
        assert_eq!(
            expected.pixels,
            Pixels::Rgba8 {
                data: vec![red, 0, 0, 255]
            }
        );
        assert_eq!(production_resize(&request).unwrap(), expected);
    }
}
