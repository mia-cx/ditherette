//! Emit public fixtures directly from the frozen oracle; no production implementation is used.

use ditherette_wasm::{
    image::contracts::PaletteEntry,
    spec::{contract::request::*, dither::error_diffusion::diffuse},
};

fn main() {
    let data: Vec<u8> = (0..35)
        .flat_map(|i| {
            [
                (i * 73 + 17) as u8,
                (i * 31 + 99) as u8,
                (i * 117 + 41) as u8,
                [0, 127, 128, 255][i as usize % 4],
            ]
        })
        .collect();
    let palette = [
        PaletteEntry::Color { rgb: [0; 3] },
        PaletteEntry::Color { rgb: [181, 31, 91] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Color { rgb: [255; 3] },
        PaletteEntry::Transparent {},
    ];
    let mut cases = Vec::new();
    for kernel in [
        Diffusion::FloydSteinberg,
        Diffusion::Sierra,
        Diffusion::SierraLite,
        Diffusion::Atkinson,
    ] {
        for feedback in [DiffusionFeedback::SrgbBytes, DiffusionFeedback::Matching] {
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
                for (alpha, placement) in [
                    (
                        AlphaPolicy::Preserve {
                            threshold: 127.9999999,
                        },
                        Placement::Everywhere {},
                    ),
                    (
                        AlphaPolicy::Premultiplied {},
                        Placement::Adaptive {
                            radius: 1,
                            threshold: 5.0,
                            softness: 10.0,
                        },
                    ),
                    (
                        AlphaPolicy::Matte { rgb: [33, 71, 109] },
                        Placement::Adaptive {
                            radius: 3,
                            threshold: 100.0,
                            softness: 0.0,
                        },
                    ),
                ] {
                    let dither = DitherPolicy::Diffusion {
                        kernel,
                        feedback,
                        strength: 0.75,
                        serpentine: cases.len() % 2 == 1,
                        placement,
                    };
                    let result = diffuse(DitherQuantizeRequest {
                        quantize: QuantizeRequest {
                            version: 1,
                            source: Source {
                                width: 5,
                                height: 7,
                                data: &data,
                            },
                            palette: &palette,
                            alpha,
                            matching,
                        },
                        dither,
                    })
                    .unwrap();
                    cases.push(serde_json::json!({"matching": matching, "alpha": alpha, "dither": dither, "indices": result.indices.data(), "warnings": result.warnings}));
                }
            }
        }
    }
    println!(
        "{}",
        serde_json::json!({"reference": "cef2b60a635fd43c3b8e7cb880b5c92fe77d640b", "source": {"width": 5, "height": 7, "data": data}, "palette": palette, "cases": cases})
    );
}
