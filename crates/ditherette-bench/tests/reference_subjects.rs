use ditherette_bench::verification::{
    content_digest, input_digest, settings_digest, verify_three_way, VerificationBounds,
    VerificationStatus,
};
use ditherette_bench_api::{verification::*, ConformanceBenchSubject, PixelFormat};
use ditherette_wasm::{
    bench_subjects::{
        reference::{ReferenceFn, ReferenceRequest},
        BenchSubject,
    },
    spec::contract::request::*,
};

fn subject(id: &str) -> ConformanceBenchSubject<ReferenceFn> {
    ditherette_wasm::bench_subjects()
        .into_iter()
        .find_map(|subject| match subject {
            BenchSubject::Conformance(subject) if subject.descriptor.id.as_str() == id => {
                Some(subject)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing callable reference {id}"))
}

fn case(request: ReferenceRequest<'_>) -> VerificationCase<ReferenceRequest<'_>> {
    let source = request.source();
    VerificationCase {
        identity: CaseIdentity {
            semantics: request.semantics(),
            input: input_digest(
                Dimensions {
                    width: source.width,
                    height: source.height,
                },
                source.data,
            ),
            settings: settings_digest(&request).unwrap(),
            output: request.dimensions().unwrap(),
        },
        request,
    }
}

#[test]
fn seven_discoverable_color_pairs_keep_packed_coordinates_alpha_and_f32_inverse() {
    let rgba = [
        0, 0, 0, 0, 255, 255, 255, 17, 255, 0, 0, 255, 10, 11, 128, 127,
    ];
    let source = Source {
        width: 4,
        height: 1,
        data: &rgba,
    };
    for (name, space) in [
        ("srgb", WorkingSpace::Srgb),
        ("linear-rgb", WorkingSpace::LinearRgb),
        ("oklab", WorkingSpace::Oklab),
        ("oklch", WorkingSpace::Oklch),
        ("cielab", WorkingSpace::Cielab),
        ("cielch", WorkingSpace::Cielch),
        ("ycbcr", WorkingSpace::Ycbcr),
    ] {
        let subject = subject(&format!("spec:color:{name}:f32-roundtrip-v1"));
        assert_eq!(subject.operation, Operation::Color);
        assert_eq!(
            subject.descriptor.capabilities.pixel_formats,
            [PixelFormat::Color32]
        );
        let output = (subject.run)(&ReferenceRequest::Color { source, space }).unwrap();
        let Pixels::Color {
            coordinates,
            alpha,
            rendered_rgba,
            ..
        } = output.pixels
        else {
            panic!("color result required")
        };
        assert_eq!(coordinates.len(), 12);
        assert_eq!(alpha, [0, 17, 255, 127]);
        assert_eq!(rendered_rgba.unwrap(), rgba);
        if space == WorkingSpace::Srgb || space == WorkingSpace::LinearRgb {
            assert_eq!(&coordinates[..6], &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        }
        let wrong_space = if space == WorkingSpace::Srgb {
            WorkingSpace::LinearRgb
        } else {
            WorkingSpace::Srgb
        };
        assert!((subject.run)(&ReferenceRequest::Color {
            source,
            space: wrong_space
        })
        .is_err());
    }
}

#[test]
fn concrete_reference_records_do_not_fill_missing_production_roles() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    let palette = [
        PaletteEntry::Transparent {},
        PaletteEntry::Color {
            rgb: [255, 255, 255],
        },
    ];
    let request = ReferenceRequest::Processing(Request::Quantize(QuantizeRequest {
        version: 1,
        source: Source {
            width: 2,
            height: 1,
            data: &[0, 0, 0, 0, 255, 255, 255, 255],
        },
        palette: &palette,
        alpha: AlphaPolicy::Preserve { threshold: 1.0 },
        matching: MatchPolicy::SrgbEuclidean,
    }));
    let case = case(request);
    let registered = subject("spec:quantize:request:v1");
    let reference = VerificationSubject {
        identity: ImplementationIdentity {
            subject: registered.descriptor.id.to_string(),
            artifact: ArtifactIdentity {
                revision: "a".repeat(40),
                content: content_digest(b"explicit fixture artifact"),
            },
        },
        semantics: case.identity.semantics.clone(),
        run: registered.run,
    }
    .evaluate(&case)
    .unwrap();
    let Pixels::Indexed8 {
        indices,
        palette_rgba,
        transparent_index,
    } = &reference.output.pixels
    else {
        panic!("indexed output required")
    };
    assert_eq!(indices, &[0, 1]);
    assert_eq!(palette_rgba, &[0, 0, 0, 0, 255, 255, 255, 255]);
    assert_eq!(*transparent_index, Some(0));
    let report = verify_three_way(
        &case.identity,
        &ThreeWayOutputs {
            reference_state: ReferenceState::PreFreeze,
            reference: Some(reference),
            accepted: None,
            candidate: None,
        },
        VerificationBounds::exact(),
    );
    assert_eq!(report.status, VerificationStatus::Incomplete);
    assert!(!report.release_conformant());
}

#[test]
fn settings_identity_binds_independent_perturb_and_matching_spaces() {
    use ditherette_wasm::image::contracts::PaletteEntry;
    let palette = [PaletteEntry::Color { rgb: [0, 0, 0] }];
    let source = Source {
        width: 1,
        height: 1,
        data: &[128, 128, 128, 255],
    };
    let mut request = DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source,
            palette: &palette,
            alpha: AlphaPolicy::Premultiplied,
            matching: MatchPolicy::CielabEuclidean,
        },
        dither: DitherPolicy::Separable {
            perturb: PerturbPolicy {
                field: Field::Random { seed: 7 },
                space: WorkingSpace::Srgb,
                strength: 1.0,
                placement: Placement::Everywhere,
            },
        },
    };
    let original = case(ReferenceRequest::Processing(Request::DitherAndQuantize(
        request,
    )));
    if let DitherPolicy::Separable { perturb } = &mut request.dither {
        perturb.space = WorkingSpace::Oklab;
    }
    let changed = case(ReferenceRequest::Processing(Request::DitherAndQuantize(
        request,
    )));
    assert_eq!(original.identity.semantics.space, Some(ColorSpace::Cielab));
    assert_eq!(original.identity.semantics, changed.identity.semantics);
    assert_ne!(original.identity.settings, changed.identity.settings);
    request.quantize.matching = MatchPolicy::SrgbEuclidean;
    let matching = case(ReferenceRequest::Processing(Request::DitherAndQuantize(
        request,
    )));
    assert_ne!(changed.identity.settings, matching.identity.settings);
    assert_ne!(
        changed.identity.semantics.space,
        matching.identity.semantics.space
    );
}
