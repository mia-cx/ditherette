//! Runs the same external contract cases against separate copied types.

use serde_json::{json, Value};

macro_rules! contract_cases {
    ($module:ident) => {{
        use ditherette_wasm::$module::contract::{error::ErrorCode, request::*};
        use ditherette_wasm::image::contracts::PaletteEntry;
        let bytes = [13, 29, 71, 0];
        let source = Source { width: 1, height: 1, data: &bytes };
        let palette = [PaletteEntry::Transparent {}];
        let output = Output { width: 2, height: 3, resize: ResizePolicy::Nearest { anchor: Anchor::BottomRight } };
        let alpha = AlphaPolicy::Preserve { threshold: 127.5 };
        let quantize = QuantizeRequest { version: 1, source, palette: &palette, alpha, matching: MatchPolicy::OklchHueArc };
        let perturb = PerturbPolicy { field: Field::Random { seed: u32::MAX }, space: WorkingSpace::Cielch, strength: f32::MAX, placement: Placement::Everywhere {} };
        let recipe = RecipeV1 { version: 1, output, alpha, matching: quantize.matching, dither: DitherPolicy::None {} };
        let requests = [
            Request::Process(ProcessRequest { source, palette: &palette, recipe }),
            Request::Resize(ResizeRequest { version: 1, source, output }),
            Request::Perturb(PerturbRequest { version: 1, source, perturb }),
            Request::Quantize(quantize),
            Request::DitherAndQuantize(DitherQuantizeRequest { quantize, dither: DitherPolicy::Yliluoma { size: BayerSize::Sixteen, placement: Placement::Everywhere {} } }),
            Request::Resize(ResizeRequest { version: 2, source, output }),
            Request::Resize(ResizeRequest { version: 1, source: Source { data: &bytes[..3], ..source }, output }),
            Request::Resize(ResizeRequest { version: 1, source, output: Output { width: 0, ..output } }),
            Request::Quantize(QuantizeRequest { palette: &[], ..quantize }),
            Request::Perturb(PerturbRequest { version: 1, source, perturb: PerturbPolicy { strength: f32::NAN, ..perturb } }),
        ];
        let mut observations = Vec::new();
        for request in requests {
            observations.push(match request.validate() {
                Ok(layout) => {
                    assert_eq!(layout.source.data().as_ptr(), bytes.as_ptr());
                    json!({"source": layout.source.data(), "output": [layout.output.width(), layout.output.height()]})
                }
                Err(error) => json!({"error": error, "display": error.to_string()}),
            });
        }
        for (width, height) in [(32768, 2048), (32768, 2049), (32769, 1), (0, 1), (u32::MAX, u32::MAX)] {
            observations.push(match validate_dimensions(width, height, MAX_SOURCE_SIDE, "source", ErrorCode::InvalidImage) {
                Ok(dimensions) => json!([dimensions.width(), dimensions.height()]),
                Err(error) => json!(error),
            });
        }
        for tag in ["srgb-euclidean", "oklch-circular-hue", "oklch-hue-arc", "cielch-circular-hue", "cielch-hue-arc", "cielab-ciede2000", "srgb-ciede2000"] {
            observations.push(match parse_match(tag, "recipe.match") {
                Ok(policy) => json!([serde_json::to_value(policy).unwrap(), serde_json::to_value(policy.space()).unwrap()]),
                Err(error) => json!(error),
            });
        }
        for encoded in [serde_json::to_string(&recipe).unwrap(), "{}".into(), r#"{"version":1,"threadCount":4}"#.into()] {
            observations.push(match decode_recipe(&encoded) {
                Ok(recipe) => json!(recipe),
                Err(error) => json!(error),
            });
        }
        observations
    }};
}

#[test]
fn requests_wire_tags_validation_order_and_diagnostics_match() {
    let expected: Vec<Value> = contract_cases!(spec);
    let actual: Vec<Value> = contract_cases!(prod);
    assert_eq!(actual, expected);
    assert_eq!(actual[5]["error"]["code"], "invalid-request");
    assert_eq!(actual[6]["error"]["path"], "source.data");
    assert_eq!(actual[9]["error"]["path"], "perturb.strength");
}

macro_rules! initialization_cases {
    ($module:ident) => {{
        use ditherette_wasm::$module::contract::{lifecycle::*, request::*};
        let mut observations = Vec::new();
        for threads in [Threads::Disabled, Threads::Preferred, Threads::Required] {
            for memory_limit_bytes in [
                0,
                1,
                DEFAULT_MEMORY_LIMIT_BYTES,
                MAX_MEMORY_LIMIT_BYTES,
                MAX_MEMORY_LIMIT_BYTES + 1,
            ] {
                let options = InitOptions {
                    threads,
                    memory_limit_bytes,
                };
                observations.push(format!("{:?}", options.validate()));
                observations.push(format!("{:?}", options.cache_limit_bytes()));
                for required in [0, 1, memory_limit_bytes, memory_limit_bytes + 1] {
                    observations.push(format!("{:?}", options.preflight(required)));
                }
                for capable in [false, true] {
                    for threaded in [false, true] {
                        for scalar in [false, true] {
                            observations.push(format!(
                                "{:?}",
                                initialize(options, capable, threaded, scalar)
                            ));
                        }
                    }
                }
            }
        }
        observations
    }};
}

#[test]
fn initialization_selection_and_budget_boundaries_match() {
    assert_eq!(initialization_cases!(prod), initialization_cases!(spec));
}

macro_rules! lifecycle_cases {
    ($module:ident) => {{
        use ditherette_wasm::$module::contract::lifecycle::*;
        let mut instance = InstanceModel::default();
        let mut independent = InstanceModel::default();
        let mut observations = Vec::new();
        macro_rules! record {
            ($call:expr) => {
                observations.push(format!("{:?}", $call));
            };
        }
        let progress = |stage| Progress {
            stage,
            completed: Some(1),
            total: Some(1),
        };
        record!(instance.finish());
        record!(instance.begin(true));
        record!(instance.begin(false));
        record!(instance.dispose());
        record!(instance.report(progress(Stage::Complete), 0));
        record!(instance.report(progress(Stage::Resize), 0));
        record!(instance.dispose());
        record!(instance.callback_succeeded());
        record!(instance.report(progress(Stage::Resize), 49));
        record!(instance.report(progress(Stage::Resize), 50));
        record!(instance.callback_failed());
        record!(instance.begin(true));
        record!(instance.output_ready());
        record!(instance.finish());
        record!(instance.report(progress(Stage::Complete), 100));
        record!(instance.callback_succeeded());
        record!(instance.report(progress(Stage::Resize), 151));
        record!(instance.finish());
        record!(instance.begin(false));
        record!(instance.fail());
        record!(instance.begin(false));
        record!(instance.output_ready());
        record!(instance.finish());
        record!(instance.dispose());
        record!(instance.dispose());
        record!(instance.begin(false));
        record!(independent.begin(false));
        record!(independent.output_ready());
        record!(independent.finish());
        observations
    }};
}

#[test]
fn lifecycle_reentry_callback_failure_completion_and_disposal_match() {
    let expected = lifecycle_cases!(spec);
    let actual = lifecycle_cases!(prod);
    assert_eq!(actual, expected);
    assert_eq!(actual[8], "Ok(false)");
    assert_eq!(actual[9], "Ok(true)");
    assert!(actual[25].contains("Disposed"));
    assert_eq!(actual[28], "Ok(())");
}
