#[path = "../examples/support/blue_noise.rs"]
mod blue_noise;

use ditherette_bench::paired::browser::{Anchor, PublicOperation, Support};
use ditherette_bench_api::verification::*;
use ditherette_bench_oracle::OracleRequest;
use ditherette_wasm::bench_subjects::{bench_subjects, BenchSubject};
use serde_json::json;

#[test]
fn isolated_oracle_preserves_existing_case_identities_and_exact_native_outputs() {
    let (source, rgba) = blue_noise::fixture();
    let registry = bench_subjects();
    for (_, native) in blue_noise::recipes() {
        let operation = blue_noise::public(&native);
        let identity = operation.identity(source, &rgba, source).unwrap();
        let request: OracleRequest = serde_json::from_value(json!({ "source":source, "rgba":rgba, "output":source, "operation":operation, "identity":identity })).unwrap();
        assert_eq!(request.case_identity().unwrap(), identity);
        let BenchSubject::Conformance(reference) = registry
            .iter()
            .find(|s| s.descriptor().id.as_str() == operation.reference_subject())
            .unwrap()
        else {
            panic!("typed reference")
        };
        let expected = (reference.run)(
            &operation
                .processing_request(source, &rgba)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(request.execute().unwrap().output, expected);
        for mutated in ["rgba", "identity", "operation"] {
            let mut value = json!({ "source":source, "rgba":rgba, "output":source, "operation":operation, "identity":identity });
            match mutated {
                "rgba" => value["rgba"][0] = json!(0),
                "identity" => value["identity"]["settings"][0] = json!(255),
                _ => {
                    let settings = &mut value["operation"]["settings"];
                    if settings.get("perturb").is_some() {
                        settings["perturb"]["strength"] = json!(0.5);
                    } else {
                        settings["strength"] = json!(0.5);
                    }
                }
            }
            assert!(serde_json::from_value::<OracleRequest>(value)
                .unwrap()
                .execute()
                .is_err());
        }
    }
}

#[test]
fn isolated_oracle_binds_every_current_resize_identity() {
    let source = Dimensions {
        width: 2,
        height: 1,
    };
    let output = Dimensions {
        width: 3,
        height: 2,
    };
    let rgba = [10, 20, 30, 40, 90, 100, 110, 120];
    for operation in [
        PublicOperation::ResizeNearest {
            anchor: Anchor::Center,
        },
        PublicOperation::ResizeArea {},
        PublicOperation::ResizeBilinear {
            anchor: Anchor::TopLeft,
        },
        PublicOperation::ResizeBicubic {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
        PublicOperation::ResizeLanczos2 {
            anchor: Anchor::Bottom,
            support: Support::ScaleAware,
        },
        PublicOperation::ResizeLanczos3 {
            anchor: Anchor::Center,
            support: Support::Fixed,
        },
    ] {
        let identity = operation.identity(source, &rgba, output).unwrap();
        let request: OracleRequest = serde_json::from_value(json!({"source":source,"rgba":rgba,"output":output,"operation":operation,"identity":identity})).unwrap();
        assert_eq!(request.execute().unwrap().case, identity);
    }
}

#[test]
fn diffusion_and_yliluoma_wire_settings_match_the_shared_typed_reference() {
    use ditherette_bench::verification::{input_digest, settings_digest};
    use ditherette_wasm::{
        bench_subjects::reference::ReferenceRequest, spec::contract::request as s,
    };
    let source = Dimensions {
        width: 2,
        height: 1,
    };
    let rgba = [73, 85, 65, 70, 120, 40, 190, 255];
    let quantize = json!({"palette":[{"kind":"color","rgb":[0,0,0]},{"kind":"color","rgb":[255,255,255]}],"alpha":{"mode":"preserve","threshold":0.5},"matching":"cielab-ciede2000"});
    let placement = json!({"mode":"everywhere"});
    let registry = bench_subjects();
    let BenchSubject::Conformance(reference) = registry
        .iter()
        .find(|s| s.descriptor().id.as_str() == "spec:dither-and-quantize:request:v1")
        .unwrap()
    else {
        panic!("typed reference")
    };
    for (operation, dither) in [
        (
            json!({"operation":"yliluoma","settings":{"quantize":quantize,"size":"2","placement":placement}}),
            json!({"family":"yliluoma","size":"2","placement":placement}),
        ),
        (
            json!({"operation":"diffusion","settings":{"quantize":quantize,"kernel":"floyd-steinberg","feedback":"srgb-bytes","strength":0.7,"serpentine":true,"placement":placement}}),
            json!({"family":"diffusion","kernel":"floyd-steinberg","feedback":"srgb-bytes","strength":0.7,"serpentine":true,"placement":placement}),
        ),
    ] {
        let palette = serde_json::from_value::<Vec<_>>(quantize["palette"].clone()).unwrap();
        let request =
            ReferenceRequest::Processing(s::Request::DitherAndQuantize(s::DitherQuantizeRequest {
                quantize: s::QuantizeRequest {
                    version: 1,
                    source: s::Source {
                        width: 2,
                        height: 1,
                        data: &rgba,
                    },
                    palette: &palette,
                    alpha: serde_json::from_value(quantize["alpha"].clone()).unwrap(),
                    matching: s::MatchPolicy::CielabCiede2000,
                },
                dither: serde_json::from_value(dither).unwrap(),
            }));
        let identity = CaseIdentity {
            semantics: request.semantics(),
            input: input_digest(source, &rgba),
            settings: settings_digest(&request).unwrap(),
            output: source,
        };
        let oracle: OracleRequest = serde_json::from_value(json!({"source":source,"rgba":rgba,"output":source,"operation":operation,"identity":identity})).unwrap();
        assert_eq!(
            oracle.execute().unwrap().output,
            (reference.run)(&request).unwrap()
        );
    }
}

fn process_wire() -> serde_json::Value {
    json!({
        "source": {"width": 2, "height": 2},
        "rgba": [73,85,65,0,120,40,190,255,9,210,51,127,250,240,230,255],
        "output": {"width": 3, "height": 2},
        "operation": {"operation": "process", "settings": {
            "palette": [{"kind":"color","rgb":[255,255,255]},
                {"kind":"color","rgb":[0,0,0]}, {"kind":"transparent"},
                {"kind":"color","rgb":[255,255,255]}],
            "recipe": {"version":1,
                "output":{"width":3,"height":2,"resize":{"algorithm":"bicubic","anchor":"center","support":"scale-aware"}},
                "alpha":{"mode":"preserve","threshold":127.5},
                "match":"cielab-ciede2000",
                "dither":{"family":"diffusion","kernel":"floyd-steinberg","feedback":"matching",
                    "strength":0.7,"serpentine":true,
                    "placement":{"mode":"adaptive","radius":1,"threshold":5,"softness":10}}
            }
        }}
    })
}

/// Expected identities and bytes come from the existing native frozen Process adapter.
fn frozen_process(wire: &serde_json::Value) -> Result<(CaseIdentity, VerificationOutput), String> {
    use ditherette_bench::verification::{input_digest, settings_digest};
    use ditherette_wasm::{
        bench_subjects::reference::ReferenceRequest, spec::contract::request as s,
    };
    let source: Dimensions = serde_json::from_value(wire["source"].clone()).unwrap();
    let rgba: Vec<u8> = serde_json::from_value(wire["rgba"].clone()).unwrap();
    let settings = &wire["operation"]["settings"];
    let palette = serde_json::from_value::<Vec<_>>(settings["palette"].clone()).unwrap();
    let request = ReferenceRequest::Processing(s::Request::Process(s::ProcessRequest {
        source: s::Source {
            width: source.width,
            height: source.height,
            data: &rgba,
        },
        palette: &palette,
        recipe: serde_json::from_value(settings["recipe"].clone()).unwrap(),
    }));
    let identity = CaseIdentity {
        semantics: request.semantics(),
        input: input_digest(source, &rgba),
        settings: settings_digest(&request).unwrap(),
        output: request.dimensions().map_err(|e| e.to_string())?,
    };
    let registry = bench_subjects();
    let BenchSubject::Conformance(reference) = registry
        .iter()
        .find(|s| s.descriptor().id.as_str() == "spec:process:request:v1")
        .unwrap()
    else {
        panic!("typed frozen Process reference")
    };
    Ok((
        identity,
        (reference.run)(&request).map_err(|e| e.to_string())?,
    ))
}

#[test]
fn process_wire_matches_frozen_identity_resized_output_and_indexed_metadata() {
    let mut wire = process_wire();
    let (identity, expected) = frozen_process(&wire).unwrap();
    wire["identity"] = serde_json::to_value(&identity).unwrap();
    let request: OracleRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(request.case_identity().unwrap(), identity);
    let actual = request.execute().unwrap();
    assert_ne!(actual.case.output, request.source);
    assert_eq!(actual.output, expected);
    let Pixels::Indexed8 {
        indices,
        palette_rgba,
        transparent_index,
    } = actual.output.pixels
    else {
        panic!("Process must publish indexed output")
    };
    assert_eq!(indices.len(), 6);
    assert_eq!(palette_rgba.len(), 16);
    assert_eq!(transparent_index, Some(2));
}

#[test]
fn process_rejects_declared_output_that_disagrees_with_its_recipe() {
    let mut wire = process_wire();
    wire["identity"] = serde_json::to_value(frozen_process(&wire).unwrap().0).unwrap();
    wire["output"] = wire["source"].clone();
    let request: OracleRequest = serde_json::from_value(wire).unwrap();
    assert!(request.case_identity().is_err());
    assert!(request.execute().is_err());
}

#[test]
fn process_preserves_every_resize_and_dither_family_with_palette_warnings() {
    let mut dithers = vec![json!({"family":"none"})];
    for field in [
        json!({"algorithm":"bayer","size":"4"}),
        json!({"algorithm":"random","seed":31}),
        json!({"algorithm":"blue-noise"}),
    ] {
        dithers.push(json!({"family":"separable","perturb":{
            "field":field,"space":"oklab","strength":0.3,"placement":{"mode":"everywhere"}
        }}));
    }
    for kernel in ["floyd-steinberg", "sierra", "sierra-lite", "atkinson"] {
        for feedback in ["srgb-bytes", "matching"] {
            dithers.push(json!({"family":"diffusion","kernel":kernel,"feedback":feedback,
                "strength":0.7,"serpentine":true,"placement":{"mode":"adaptive","radius":1,"threshold":5,"softness":10}}));
        }
    }
    dithers.push(json!({"family":"yliluoma","size":"2","placement":{"mode":"adaptive","radius":1,"threshold":5,"softness":10}}));
    let mut resizes = vec![
        json!({"algorithm":"nearest","anchor":"top-left"}),
        json!({"algorithm":"area"}),
        json!({"algorithm":"bilinear","anchor":"bottom-right"}),
        json!({"algorithm":"trilinear","anchor":"left"}),
    ];
    for algorithm in ["bicubic", "lanczos2", "lanczos3"] {
        for support in ["fixed", "scale-aware"] {
            resizes.push(json!({"algorithm":algorithm,"anchor":"center","support":support}));
        }
    }
    let mut cases = 0;
    for resize in resizes {
        for dither in &dithers {
            let mut wire = process_wire();
            let settings = &mut wire["operation"]["settings"];
            settings["recipe"]["output"]["resize"] = resize.clone();
            settings["recipe"]["dither"] = dither.clone();
            // Rotate metadata controls without expanding the cross-product.
            match cases % 3 {
                0 => settings["recipe"]["alpha"] = json!({"mode":"premultiplied"}),
                1 => settings["recipe"]["alpha"] = json!({"mode":"matte","rgb":[17,33,71]}),
                _ => {}
            }
            let (identity, expected) = frozen_process(&wire).unwrap();
            wire["identity"] = serde_json::to_value(&identity).unwrap();
            let oracle: OracleRequest = serde_json::from_value(wire).unwrap();
            let actual = oracle.execute().unwrap();
            assert_eq!(actual.case, identity);
            assert_eq!(actual.output, expected, "{resize} {dither}");
            cases += 1;
        }
    }
    assert_eq!(cases, 130);
    for palette in [
        vec![json!({"kind":"transparent"}); 257],
        vec![json!({"kind":"color","rgb":[73,85,65]}); 257],
    ] {
        let mut wire = process_wire();
        wire["operation"]["settings"]["palette"] = json!(palette);
        let (identity, expected) = frozen_process(&wire).unwrap();
        assert_eq!(expected.warnings.len(), 2);
        wire["identity"] = json!(identity);
        assert_eq!(
            serde_json::from_value::<OracleRequest>(wire)
                .unwrap()
                .execute()
                .unwrap()
                .output,
            expected
        );
    }
}

#[test]
fn process_identity_binds_normalized_recipe_palette_source_and_output() {
    let mut wire = process_wire();
    let identity = frozen_process(&wire).unwrap().0;
    wire["identity"] = json!(identity);
    let mut normalized = wire.clone();
    normalized["operation"]["settings"]["recipe"]["dither"]["strength"] = json!(0.7000000001);
    let normalized: OracleRequest = serde_json::from_value(normalized).unwrap();
    // Both JSON numbers normalize to the same frozen f32 control.
    assert_eq!(normalized.execute().unwrap().case, identity);

    for (pointer, replacement) in [
        ("/rgba/0", json!(74)),
        ("/operation/settings/palette/0/rgb/0", json!(128)),
        ("/operation/settings/recipe/output/width", json!(4)),
        ("/operation/settings/recipe/output/height", json!(3)),
        (
            "/operation/settings/recipe/output/resize/anchor",
            json!("left"),
        ),
        (
            "/operation/settings/recipe/output/resize/support",
            json!("fixed"),
        ),
        (
            "/operation/settings/recipe/output/resize/algorithm",
            json!("lanczos2"),
        ),
        (
            "/operation/settings/recipe/alpha/threshold",
            json!(127.5000000000001),
        ),
        (
            "/operation/settings/recipe/alpha",
            json!({"mode":"matte","rgb":[17,33,71]}),
        ),
        ("/operation/settings/recipe/match", json!("oklab-euclidean")),
        (
            "/operation/settings/recipe/dither/kernel",
            json!("atkinson"),
        ),
        (
            "/operation/settings/recipe/dither/feedback",
            json!("srgb-bytes"),
        ),
        ("/operation/settings/recipe/dither/strength", json!(0.5)),
        ("/operation/settings/recipe/dither/serpentine", json!(false)),
        (
            "/operation/settings/recipe/dither/placement/radius",
            json!(2),
        ),
        (
            "/operation/settings/recipe/dither/placement/threshold",
            json!(6),
        ),
        (
            "/operation/settings/recipe/dither/placement/softness",
            json!(11),
        ),
        (
            "/operation/settings/recipe/dither",
            json!({"family":"none"}),
        ),
        ("/source", json!({"width":4,"height":1})),
    ] {
        let mut changed = wire.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        changed["output"] = json!({
            "width": changed["operation"]["settings"]["recipe"]["output"]["width"],
            "height": changed["operation"]["settings"]["recipe"]["output"]["height"]
        });
        let expected_identity = frozen_process(&changed).unwrap().0;
        assert_ne!(expected_identity, identity, "{pointer}");
        let oracle: OracleRequest = serde_json::from_value(changed).unwrap();
        assert_eq!(oracle.case_identity().unwrap(), expected_identity);
        assert!(oracle.execute().is_err(), "stale identity at {pointer}");
    }

    // A palette tail can leave output unchanged while still changing full request identity.
    let mut tail = wire.clone();
    tail["operation"]["settings"]["palette"] = json!(vec![json!({"kind":"transparent"}); 257]);
    let (before, pixels) = frozen_process(&tail).unwrap();
    tail["identity"] = json!(before);
    tail["operation"]["settings"]["palette"][256] = json!({"kind":"color","rgb":[1,2,3]});
    let (after, same_pixels) = frozen_process(&tail).unwrap();
    assert_eq!(pixels, same_pixels);
    assert_ne!(before.settings, after.settings);
    let oracle: OracleRequest = serde_json::from_value(tail).unwrap();
    assert_eq!(oracle.case_identity().unwrap(), after);
    assert!(oracle.execute().is_err());
}

#[test]
fn process_rejects_invalid_requests_and_caller_reference_overrides() {
    let mut wire = process_wire();
    let (identity, expected) = frozen_process(&wire).unwrap();
    wire["identity"] = json!(identity);
    for (pointer, replacement) in [
        ("/source/width", json!(0)),
        ("/rgba", json!([0, 0, 0, 0])),
        ("/operation/settings/palette", json!([])),
        ("/operation/settings/recipe/version", json!(2)),
        ("/operation/settings/recipe/output/width", json!(0)),
        ("/operation/settings/recipe/alpha/threshold", json!(-1)),
        ("/operation/settings/recipe/dither/strength", json!(-1)),
        (
            "/operation/settings/recipe/dither/placement/radius",
            json!(0),
        ),
    ] {
        let mut invalid = wire.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        let expected = frozen_process(&invalid).unwrap_err();
        let oracle: OracleRequest = serde_json::from_value(invalid).unwrap();
        assert_eq!(oracle.case_identity().unwrap_err(), expected, "{pointer}");
        assert_eq!(oracle.execute().unwrap_err(), expected, "{pointer}");
    }
    let mut invalid = wire.clone();
    invalid["operation"]["settings"]["recipe"]["version"] = json!(2);
    invalid["rgba"] = json!([]);
    let expected_error = frozen_process(&invalid).unwrap_err();
    assert!(expected_error.contains("recipe.version"));
    assert_eq!(
        serde_json::from_value::<OracleRequest>(invalid)
            .unwrap()
            .execute()
            .unwrap_err(),
        expected_error
    );

    for (pointer, replacement) in [
        ("/operation/settings/recipe/match", json!("cielab-rec709")),
        ("/operation/settings/recipe/dither/serpentine", json!(1)),
        (
            "/operation/settings/recipe/output/resize/anchor",
            json!("unknown"),
        ),
    ] {
        let mut invalid = wire.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        assert!(serde_json::from_value::<OracleRequest>(invalid).is_err());
    }
    // Native diagnostics remain external evidence, never an oracle input or output override.
    for key in ["native_reference", "reference", "expected_output"] {
        let mut supplied = wire.clone();
        supplied[key] = json!(expected);
        assert!(serde_json::from_value::<OracleRequest>(supplied).is_err());
    }
    let actual = serde_json::from_value::<OracleRequest>(wire)
        .unwrap()
        .execute()
        .unwrap();
    assert_eq!(actual.case, identity);
    assert_eq!(actual.output, expected);
}
