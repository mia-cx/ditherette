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
