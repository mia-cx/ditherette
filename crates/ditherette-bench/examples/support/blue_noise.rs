//! Fixed S27 recipes and image, shared by declarations and untimed fixtures.
use ditherette_bench::paired::{
    browser::PublicOperation, fields::*, native::NativeOperation, quantize::*,
};
use ditherette_bench_api::verification::Dimensions;

pub fn fixture() -> (Dimensions, Vec<u8>) {
    let source = Dimensions {
        width: 65,
        height: 33,
    };
    let rgba = (0..source.height)
        .flat_map(|y| {
            (0..source.width).flat_map(move |x| {
                [
                    (x * 17 + y * 7 + 11) as u8,
                    (y * 31 + x * 3 + 23) as u8,
                    ((x ^ y) + 47) as u8,
                    (x * 43 + y * 19) as u8,
                ]
            })
        })
        .collect();
    (source, rgba)
}

pub fn recipes() -> Vec<(String, NativeOperation)> {
    let adaptive = |radius| Placement::Adaptive {
        radius,
        threshold: 10.0,
        softness: 5.0,
    };
    let policy = |space, placement| PerturbPolicy {
        field: Field::BlueNoise {},
        space,
        strength: 0.7,
        placement,
    };
    let mut recipes = vec![
        (
            "blue-noise-srgb-everywhere".into(),
            NativeOperation::Perturb {
                settings: policy(WorkingSpace::Srgb, Placement::Everywhere {}),
            },
        ),
        (
            "blue-noise-oklab-adaptive2".into(),
            NativeOperation::Perturb {
                settings: policy(WorkingSpace::Oklab, adaptive(2)),
            },
        ),
        (
            "blue-noise-ycbcr-everywhere".into(),
            NativeOperation::Perturb {
                settings: policy(WorkingSpace::Ycbcr, Placement::Everywhere {}),
            },
        ),
    ];
    let mut palette: Vec<_> = (0..63)
        .map(|i| PaletteEntry::Color {
            rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
        })
        .collect();
    palette.push(PaletteEntry::Transparent {});
    recipes.push((
        "blue-noise-ycbcr-adaptive1-oklch-hue-arc-palette64".into(),
        NativeOperation::Separable {
            settings: SeparableSettings {
                perturb: policy(WorkingSpace::Ycbcr, adaptive(1)),
                quantize: QuantizeSettings {
                    palette,
                    alpha: AlphaPolicy::Preserve { threshold: 0.5 },
                    matching: MatchPolicy::OklchHueArc,
                },
            },
        },
    ));
    recipes
}

pub fn public(native: &NativeOperation) -> PublicOperation {
    match native {
        NativeOperation::Perturb { settings } => PublicOperation::Perturb {
            settings: *settings,
        },
        NativeOperation::Separable { settings } => PublicOperation::Separable {
            settings: settings.clone(),
        },
        _ => unreachable!("S27 complete recipe"),
    }
}
