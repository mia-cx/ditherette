//! Untimed frozen public Yliluoma fixtures. No production code executes here.

use ditherette_wasm::{
    image::contracts::PaletteEntry,
    spec::{contract::request::*, dither::yiluoma::dither_yiluoma},
};

fn emit(request: DitherQuantizeRequest<'_>) -> serde_json::Value {
    let image = dither_yiluoma(request).unwrap();
    let q = request.quantize;
    serde_json::json!({
        "request": {"version": 1, "source": {"width": q.source.width, "height": q.source.height, "data": q.source.data}, "palette": q.palette, "alpha": q.alpha, "matching": q.matching, "dither": request.dither},
        "output": {"width": q.source.width, "height": q.source.height, "indices": image.indices.data(), "palette": {"rgba": image.palette.rgba, "transparentIndex": image.palette.transparent_index}, "warnings": image.warnings},
    })
}

fn main() {
    let data = [
        64, 64, 64, 255, 17, 33, 71, 127, 255, 1, 3, 128, 255, 0, 0, 0,
    ];
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [128; 3] },
        PaletteEntry::Color { rgb: [64; 3] },
        PaletteEntry::Transparent {},
    ];
    let base = DitherQuantizeRequest {
        quantize: QuantizeRequest {
            version: 1,
            source: Source {
                width: 2,
                height: 2,
                data: &data,
            },
            palette: &palette,
            alpha: AlphaPolicy::Preserve {
                threshold: 127.9999999,
            },
            matching: MatchPolicy::SrgbEuclidean,
        },
        dither: DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Everywhere {},
        },
    };
    let mut cases = Vec::new();
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
        for size in [
            BayerSize::Two,
            BayerSize::Four,
            BayerSize::Eight,
            BayerSize::Sixteen,
        ] {
            for placement in [
                Placement::Everywhere {},
                Placement::Adaptive {
                    radius: 1,
                    threshold: 5.0,
                    softness: 10.0,
                },
            ] {
                for alpha in [
                    AlphaPolicy::Preserve {
                        threshold: 127.9999999,
                    },
                    AlphaPolicy::Premultiplied {},
                    AlphaPolicy::Matte { rgb: [29, 71, 211] },
                ] {
                    cases.push(emit(DitherQuantizeRequest {
                        quantize: QuantizeRequest {
                            matching,
                            alpha,
                            ..base.quantize
                        },
                        dither: DitherPolicy::Yliluoma { size, placement },
                    }));
                }
            }
        }
    }
    for (width, height) in [(1, 17), (17, 2), (33, 17)] {
        let bytes: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 73) as u8,
                    (i * 31 + 19) as u8,
                    (i * 17 + 113) as u8,
                    255,
                ]
            })
            .collect();
        cases.push(emit(DitherQuantizeRequest {
            quantize: QuantizeRequest {
                source: Source {
                    width,
                    height,
                    data: &bytes,
                },
                ..base.quantize
            },
            dither: DitherPolicy::Yliluoma {
                size: BayerSize::Sixteen,
                placement: Placement::Adaptive {
                    radius: 2,
                    threshold: 5.0,
                    softness: 10.0,
                },
            },
        }));
    }
    for entries in [
        vec![PaletteEntry::Transparent {}],
        vec![PaletteEntry::Color { rgb: [0; 3] }],
        (0..257)
            .map(|i| PaletteEntry::Color { rgb: [i as u8; 3] })
            .collect(),
    ] {
        cases.push(emit(DitherQuantizeRequest {
            quantize: QuantizeRequest {
                palette: &entries,
                ..base.quantize
            },
            ..base
        }));
    }
    let gray = [64, 64, 64, 255].repeat(4);
    cases.push(emit(DitherQuantizeRequest {
        quantize: QuantizeRequest {
            source: Source {
                width: 2,
                height: 2,
                data: &gray,
            },
            ..base.quantize
        },
        dither: DitherPolicy::Yliluoma {
            size: BayerSize::Two,
            placement: Placement::Adaptive {
                radius: 1,
                threshold: 100.0,
                softness: 0.0,
            },
        },
    }));
    println!(
        "{}",
        serde_json::json!({"reference": "cef2b60a635fd43c3b8e7cb880b5c92fe77d640b", "cases": cases})
    );
}
