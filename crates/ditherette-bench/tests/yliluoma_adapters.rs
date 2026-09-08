//! Untimed exactness and identity checks for the literal native benchmark adapter.

use ditherette_bench::{
    paired::{native::NativeOperation, quantize::*, yliluoma::YliluomaSettings, CallScope},
    verification::settings_digest,
};
use ditherette_bench_api::verification::{Dimensions, Operation};
use ditherette_wasm::{
    bench_subjects::{
        self, reference::ReferenceRequest, verification::indexed_output, yiluoma, BenchSubject,
    },
    spec::contract::request::{BayerSize, Placement, Request},
};

fn settings(size: usize, matching: MatchPolicy) -> YliluomaSettings {
    YliluomaSettings {
        quantize: QuantizeSettings {
            palette: (0..size)
                .map(|i| {
                    if i == 1 {
                        PaletteEntry::Transparent {}
                    } else {
                        PaletteEntry::Color {
                            rgb: [(i * 73) as u8, (i * 31 + 17) as u8, (i * 43 + 119) as u8],
                        }
                    }
                })
                .collect(),
            alpha: AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            matching,
        },
        size: BayerSize::Two,
        placement: Placement::Adaptive {
            radius: 1,
            threshold: 5.0,
            softness: 10.0,
        },
    }
}

#[test]
fn typed_literal_call_and_registry_match_all_metrics_with_small_and_large_palettes() {
    let dimensions = Dimensions {
        width: 2,
        height: 1,
    };
    let rgba = [71, 128, 193, 255, 255, 0, 0, 0];
    let registry = bench_subjects::bench_subjects();
    for matching in [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ] {
        for palette_size in [2, 16, 256] {
            let operation = NativeOperation::Yliluoma {
                settings: settings(palette_size, matching),
            };
            let request = operation.reference_request(dimensions, &rgba).unwrap();
            assert_eq!(operation.scope(), CallScope::NativeCompleteCall);
            let run = |id: &str| {
                let BenchSubject::Conformance(subject) = registry
                    .iter()
                    .find(|entry| entry.descriptor().id.as_str() == id)
                    .unwrap()
                else {
                    panic!("typed subject")
                };
                assert_eq!(subject.operation, Operation::DitherAndQuantize);
                if id == yiluoma::YLILUOMA_SUBJECT {
                    assert_eq!(
                        subject.descriptor.default_oracle.as_ref().unwrap().as_str(),
                        operation.reference_subject()
                    );
                }
                (subject.run)(&request).unwrap()
            };
            let expected = run(operation.reference_subject());
            assert_eq!(run(yiluoma::YLILUOMA_SUBJECT), expected);
            let mapped = yiluoma::yiluoma_request(&request).unwrap();
            assert_eq!(mapped.quantize.source.data.as_ptr(), rgba.as_ptr());
            let actual =
                yiluoma::yiluoma_function(yiluoma::YLILUOMA_SUBJECT).unwrap()(mapped).unwrap();
            assert_eq!(indexed_output(&actual), expected);
            assert_eq!(rgba, [71, 128, 193, 255, 255, 0, 0, 0]);
        }
    }
}

#[test]
fn normalized_identity_binds_every_mix_control_and_rejects_invalid_requests() {
    let dimensions = Dimensions {
        width: 1,
        height: 1,
    };
    let rgba = [128; 4];
    let original = settings(3, MatchPolicy::SrgbEuclidean);
    let identity = |settings: YliluomaSettings| {
        NativeOperation::Yliluoma { settings }
            .identity(dimensions, &rgba)
            .unwrap()
    };
    let expected = identity(original.clone());
    let public = ditherette_bench::paired::browser::PublicOperation::Yliluoma {
        settings: original.clone(),
    };
    assert_eq!(
        public.identity(dimensions, &rgba, dimensions).unwrap(),
        expected
    );
    assert_eq!(
        public.reference_subject(),
        "spec:dither-and-quantize:request:v1"
    );
    assert_eq!(
        public.subject(ditherette_bench::paired::browser::BrowserBackend::Package),
        "public:dither-and-quantize:yliluoma:package"
    );
    let encoded = serde_json::to_value(&public).unwrap();
    assert_eq!(
        serde_json::from_value::<ditherette_bench::paired::browser::PublicOperation>(encoded)
            .unwrap(),
        public
    );
    let mut changes = Vec::new();
    let mut changed = original.clone();
    changed.size = BayerSize::Sixteen;
    changes.push(changed);
    let mut changed = original.clone();
    changed.quantize.matching = MatchPolicy::OklchHueArc;
    changes.push(changed);
    let mut changed = original.clone();
    changed.quantize.alpha = AlphaPolicy::Premultiplied {};
    changes.push(changed);
    let mut changed = original.clone();
    changed.quantize.palette.swap(0, 2);
    changes.push(changed);
    for placement in [
        Placement::Everywhere {},
        Placement::Adaptive {
            radius: 2,
            threshold: 5.0,
            softness: 10.0,
        },
        Placement::Adaptive {
            radius: 1,
            threshold: 6.0,
            softness: 10.0,
        },
        Placement::Adaptive {
            radius: 1,
            threshold: 5.0,
            softness: 11.0,
        },
    ] {
        let mut changed = original.clone();
        changed.placement = placement;
        changes.push(changed);
    }
    for changed in changes {
        assert_ne!(identity(changed).settings, expected.settings);
    }
    let operation = NativeOperation::Yliluoma {
        settings: original.clone(),
    };
    assert_eq!(
        serde_json::from_str::<NativeOperation>(&serde_json::to_string(&operation).unwrap())
            .unwrap(),
        operation
    );
    assert!(operation.reference_request(dimensions, &rgba[..3]).is_err());
    let mut invalid = original.clone();
    invalid.placement = Placement::Adaptive {
        radius: 0,
        threshold: 5.0,
        softness: 10.0,
    };
    assert!(NativeOperation::Yliluoma { settings: invalid }
        .identity(dimensions, &rgba)
        .is_err());
    let request = original
        .quantize
        .reference_request(dimensions, &rgba)
        .unwrap();
    assert!(yiluoma::yiluoma_request(&request).is_err());
    assert!(yiluoma::yiluoma_function("spec:dither-and-quantize:request:v1").is_none());
    let ReferenceRequest::Processing(Request::Quantize(quantize)) = request else {
        panic!("typed quantize")
    };
    let request = ReferenceRequest::Processing(Request::DitherAndQuantize(
        ditherette_wasm::spec::contract::request::DitherQuantizeRequest {
            quantize,
            dither: ditherette_wasm::spec::contract::request::DitherPolicy::None {},
        },
    ));
    assert!(yiluoma::yiluoma_request(&request).is_err());
    assert_ne!(settings_digest(&request).unwrap(), expected.settings);
}
