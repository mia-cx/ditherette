//! Emit public fixtures for the `dither_modes` kernels and tiles straight from the reference.
//! Run with cargo run --example generate_dither_modes_conformance; this never calls production.

use ditherette_wasm::{
    image::contracts::PaletteEntry,
    spec::{
        contract::request::{
            AlphaPolicy, DiffusionFeedback, MatchPolicy, Placement, QuantizeRequest, Source,
            WorkingSpace,
        },
        dither_modes::{
            diffusion::{diffuse, DiffusionPolicy, Kernel},
            ordered::{dither_and_quantize, perturb, Tile, TilePerturbPolicy},
        },
    },
};
use serde_json::json;

const WIDTH: u32 = 5;
const HEIGHT: u32 = 7;

fn main() {
    let data: Vec<u8> = (0..WIDTH * HEIGHT)
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
    let source = Source {
        width: WIDTH,
        height: HEIGHT,
        data: &data,
    };
    let request = |alpha, matching| QuantizeRequest {
        version: 1,
        source,
        palette: &palette,
        alpha,
        matching,
    };
    let adaptive = Placement::Adaptive {
        radius: 1,
        threshold: 5.0,
        softness: 10.0,
    };

    let mut diffusion = Vec::new();
    for kernel in [
        Kernel::JarvisJudiceNinke,
        Kernel::Stucki,
        Kernel::Burkes,
        Kernel::TwoRowSierra,
        Kernel::Fan,
        Kernel::ShiauFan,
        Kernel::ShiauFan2,
        Kernel::Simple2d,
    ] {
        for (feedback, matching, alpha, placement, serpentine) in [
            (
                DiffusionFeedback::SrgbBytes,
                MatchPolicy::SrgbEuclidean,
                AlphaPolicy::Preserve {
                    threshold: 127.9999999,
                },
                Placement::Everywhere {},
                false,
            ),
            (
                DiffusionFeedback::Matching,
                MatchPolicy::OklabEuclidean,
                AlphaPolicy::Matte { rgb: [33, 71, 109] },
                adaptive,
                true,
            ),
        ] {
            let policy = DiffusionPolicy {
                kernel,
                strength: 0.75,
                placement,
                serpentine,
                feedback,
            };
            let result = diffuse(request(alpha, matching), policy).unwrap();
            diffusion.push(json!({
                "alpha": alpha,
                "matching": matching,
                "dither": {
                    "family": "diffusion",
                    "kernel": kernel,
                    "feedback": feedback,
                    "strength": 0.75,
                    "serpentine": serpentine,
                    "placement": placement,
                },
                "indices": result.indices.data(),
            }));
        }
    }

    let mut ordered = Vec::new();
    for tile in [
        Tile::ThreeByOne,
        Tile::FourByOne,
        Tile::FourByTwo,
        Tile::FiveByThree,
    ] {
        for (space, placement, matching) in [
            (
                WorkingSpace::Srgb,
                Placement::Everywhere {},
                MatchPolicy::SrgbEuclidean,
            ),
            (WorkingSpace::Oklab, adaptive, MatchPolicy::CielabCiede2000),
        ] {
            let policy = TilePerturbPolicy {
                tile,
                space,
                strength: 0.8,
                placement,
            };
            let alpha = AlphaPolicy::Premultiplied {};
            let rgba = perturb(source, policy).unwrap();
            let indexed = dither_and_quantize(request(alpha, matching), policy).unwrap();
            ordered.push(json!({
                "perturb": {
                    "field": { "algorithm": "ordered", "tile": tile },
                    "space": space,
                    "strength": 0.8,
                    "placement": placement,
                },
                "alpha": alpha,
                "matching": matching,
                "rgba": rgba.data(),
                "indices": indexed.indices.data(),
            }));
        }
    }

    println!(
        "{}",
        json!({
            "source": { "width": WIDTH, "height": HEIGHT, "data": data },
            "palette": palette,
            "diffusion": diffusion,
            "ordered": ordered,
        })
    );
}
