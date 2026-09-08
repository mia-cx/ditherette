use ditherette_bench::paired::{
    browser::PublicOperation, native::NativeOperation, process::ProcessSettings,
};
use ditherette_bench_api::verification::{Dimensions, Operation};
use ditherette_wasm::{
    bench_subjects::{
        self, field_calls,
        process::{CompleteCall, PROCESS_SUBJECT, STAGED_SUBJECT},
        BenchSubject,
    },
    image::contracts::PaletteEntry,
    spec::contract::request::*,
};

fn settings(resize: ResizePolicy, dither: DitherPolicy) -> ProcessSettings {
    ProcessSettings {
        palette: vec![
            PaletteEntry::Color { rgb: [0; 3] },
            PaletteEntry::Color { rgb: [255; 3] },
            PaletteEntry::Transparent {},
        ],
        recipe: RecipeV1 {
            version: 1,
            output: Output {
                width: 3,
                height: 2,
                resize,
            },
            alpha: AlphaPolicy::Preserve { threshold: 0.5 },
            matching: MatchPolicy::SrgbEuclidean,
            dither,
        },
    }
}

#[test]
fn complete_and_staged_subjects_preserve_all_resize_dither_compositions() {
    let source = Dimensions {
        width: 4,
        height: 3,
    };
    let rgba: Vec<u8> = (0..12)
        .flat_map(|n| [n * 19, n * 13, n * 7, if n % 3 == 0 { 0 } else { 255 }])
        .collect();
    let filters = [
        ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
        ResizePolicy::Area {},
        ResizePolicy::Bilinear {
            anchor: Anchor::Center,
        },
        ResizePolicy::Bicubic {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos2 {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        ResizePolicy::Lanczos3 {
            anchor: Anchor::Center,
            support: Support::ScaleAware,
        },
        ResizePolicy::Trilinear {
            anchor: Anchor::Center,
        },
    ];
    let dithers = [
        DitherPolicy::None {},
        DitherPolicy::Separable {
            perturb: PerturbPolicy {
                field: Field::BlueNoise {},
                space: WorkingSpace::Srgb,
                strength: 0.7,
                placement: Placement::Everywhere {},
            },
        },
        DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    ];
    let registry = bench_subjects::bench_subjects();
    let diffusion = [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
    ]
    .map(|kernel| DitherPolicy::Diffusion {
        kernel,
        strength: 0.7,
        placement: Placement::Everywhere {},
        serpentine: true,
        feedback: DiffusionFeedback::Matching,
    });
    for filter in filters {
        for dither in dithers.into_iter().chain(diffusion) {
            let settings = settings(filter, dither);
            let native = NativeOperation::Process {
                settings: settings.clone(),
            };
            let public = PublicOperation::Process { settings };
            let identity = native.identity(source, &rgba).unwrap();
            assert_eq!(
                identity.output,
                Dimensions {
                    width: 3,
                    height: 2
                }
            );
            assert_eq!(identity.semantics.operation, Operation::Process);
            assert_eq!(
                identity,
                public.identity(source, &rgba, identity.output).unwrap()
            );
            assert!(public.identity(source, &rgba, source).is_err());
            let request = native.reference_request(source, &rgba).unwrap();
            let oracle: ditherette_bench_oracle::OracleRequest = serde_json::from_value(
                serde_json::json!({"source":source,"rgba":rgba,"output":identity.output,"operation":public,"identity":identity}),
            ).unwrap();
            let reference = oracle.execute().unwrap();
            assert_eq!(reference.case, identity);
            let a = CompleteCall::new(&request, PROCESS_SUBJECT)
                .unwrap()
                .output(&mut field_calls::processor().unwrap())
                .unwrap();
            let b = CompleteCall::new(&request, STAGED_SUBJECT)
                .unwrap()
                .output(&mut field_calls::processor().unwrap())
                .unwrap();
            assert_eq!(a, b, "{filter:?} {dither:?}");
            for id in [PROCESS_SUBJECT, STAGED_SUBJECT, "spec:process:request:v1"] {
                let subject = registry
                    .iter()
                    .find_map(|s| match s {
                        BenchSubject::Conformance(s) if s.descriptor.id.as_str() == id => Some(s),
                        _ => None,
                    })
                    .unwrap();
                let output = (subject.run)(&request).unwrap();
                assert_eq!(output.dimensions, identity.output);
                if id == "spec:process:request:v1" {
                    assert_eq!(output, reference.output);
                }
            }
            CompleteCall::new(&request, PROCESS_SUBJECT)
                .unwrap()
                .run(&mut field_calls::processor().unwrap())
                .unwrap();
        }
    }
}

#[test]
fn process_identity_binds_output_recipe_palette_and_rejects_invalid_mapping() {
    let source = Dimensions {
        width: 1,
        height: 1,
    };
    let rgba = [1, 2, 3, 255];
    let valid = settings(
        ResizePolicy::Nearest {
            anchor: Anchor::Center,
        },
        DitherPolicy::None {},
    );
    let identity = NativeOperation::Process {
        settings: valid.clone(),
    }
    .identity(source, &rgba)
    .unwrap();
    for mutation in 0..4 {
        let mut settings = valid.clone();
        match mutation {
            0 => settings.recipe.output.width = 4,
            1 => settings.recipe.alpha = AlphaPolicy::Matte { rgb: [1, 2, 3] },
            2 => settings.recipe.output.resize = ResizePolicy::Area {},
            _ => settings.palette.reverse(),
        }
        assert_ne!(
            identity,
            NativeOperation::Process { settings }
                .identity(source, &rgba)
                .unwrap()
        );
    }
    let mut invalid = valid.clone();
    invalid.recipe.version = 2;
    assert!(invalid.reference_request(source, &rgba).is_err());
    assert!(valid.reference_request(source, &rgba[..3]).is_err());
    let reference = valid.reference_request(source, &rgba).unwrap();
    assert!(CompleteCall::new(&reference, "spec:process:request:v1").is_err());
}
