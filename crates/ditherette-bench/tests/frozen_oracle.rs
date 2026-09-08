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
