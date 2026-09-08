//! Emit untimed frozen field vectors for installed-package browser conformance.
//! Run with cargo run --example generate_field_conformance; this never calls production.

use ditherette_wasm::{
    image::{ImageDimensions, ImageView, Rgba8},
    spec::{
        contract::request::{BayerSize, Field, PerturbPolicy, Placement, WorkingSpace},
        dither::perturb::perturb,
    },
};

fn main() {
    let data = [
        255, 0, 0, 0, 17, 33, 71, 127, 128, 128, 128, 128, 0, 0, 255, 255,
    ];
    let dimensions = ImageDimensions::new(2, 2).unwrap();
    let source = ImageView::<Rgba8>::packed(&data, dimensions).unwrap();
    let mut cases = Vec::new();
    for space in [
        WorkingSpace::Srgb,
        WorkingSpace::LinearRgb,
        WorkingSpace::Oklab,
        WorkingSpace::Oklch,
        WorkingSpace::Cielab,
        WorkingSpace::Cielch,
        WorkingSpace::Ycbcr,
    ] {
        for field in [
            Field::BlueNoise {},
            Field::Bayer {
                size: BayerSize::Two,
            },
            Field::Bayer {
                size: BayerSize::Four,
            },
            Field::Bayer {
                size: BayerSize::Eight,
            },
            Field::Bayer {
                size: BayerSize::Sixteen,
            },
            Field::Random { seed: 0 },
            Field::Random { seed: u32::MAX },
        ] {
            for placement in [
                Placement::Everywhere {},
                Placement::Adaptive {
                    radius: 1,
                    threshold: 5.0,
                    softness: 10.0,
                },
            ] {
                let policy = PerturbPolicy {
                    field,
                    space,
                    strength: 0.7,
                    placement,
                };
                cases.push(serde_json::json!({"policy": policy, "rgba": perturb(source, policy).unwrap().data()}));
            }
        }
        let policy = PerturbPolicy {
            field: Field::Random { seed: 1 },
            space,
            strength: f32::MAX,
            placement: Placement::Everywhere {},
        };
        cases.push(
            serde_json::json!({"policy": policy, "rgba": perturb(source, policy).unwrap().data()}),
        );
    }
    for (width, height) in [(1, 33), (31, 2), (32, 2), (33, 33), (65, 2)] {
        let bytes: Vec<u8> = (0..width * height)
            .flat_map(|i| {
                [
                    (i * 73) as u8,
                    (i * 31 + 127) as u8,
                    (i * 17 + 255) as u8,
                    [0, 1, 127, 128, 254, 255][i as usize % 6],
                ]
            })
            .collect();
        let image =
            ImageView::<Rgba8>::packed(&bytes, ImageDimensions::new(width, height).unwrap())
                .unwrap();
        let policy = PerturbPolicy {
            field: Field::BlueNoise {},
            space: WorkingSpace::Srgb,
            strength: 0.7,
            placement: Placement::Adaptive {
                radius: 2,
                threshold: 5.0,
                softness: 10.0,
            },
        };
        cases.push(serde_json::json!({
            "source": {"width": width, "height": height, "data": bytes},
            "policy": policy,
            "rgba": perturb(image, policy).unwrap().data(),
        }));
    }
    println!(
        "{}",
        serde_json::json!({"reference": "cef2b60a635fd43c3b8e7cb880b5c92fe77d640b", "source": {"width": 2, "height": 2, "data": data}, "cases": cases})
    );
}
