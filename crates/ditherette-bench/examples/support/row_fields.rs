//! S36 complete-call recipes. Dimensions, scheduling, and trial budgets belong to the caller.
use ditherette_bench::paired::{
    browser::PublicOperation, fields::*, native::NativeOperation, quantize::*,
};

/// Pair cheap matching with perceptual matching, and independent fields with adaptive neighbors.
pub fn recipes() -> Vec<(String, NativeOperation)> {
    let mut recipes = Vec::new();
    for (name, matching, count, perturb) in [
        (
            "srgb16-random",
            MatchPolicy::SrgbEuclidean,
            16,
            PerturbPolicy {
                space: WorkingSpace::Srgb,
                strength: 0.7,
                field: Field::Random { seed: 0x12345678 },
                placement: Placement::Everywhere {},
            },
        ),
        (
            "oklab64-blue-adaptive2",
            MatchPolicy::OklabEuclidean,
            64,
            PerturbPolicy {
                space: WorkingSpace::Oklab,
                strength: 0.7,
                field: Field::BlueNoise {},
                placement: Placement::Adaptive {
                    radius: 2,
                    threshold: 0.05,
                    softness: 0.025,
                },
            },
        ),
    ] {
        let mut palette: Vec<_> = (0..count - 1)
            .map(|i| PaletteEntry::Color {
                rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
            })
            .collect();
        palette.push(PaletteEntry::Transparent {});
        let quantize = QuantizeSettings {
            palette,
            alpha: AlphaPolicy::Preserve { threshold: 127.5 },
            matching,
        };
        recipes.extend([
            (
                format!("quantize-{name}"),
                NativeOperation::Quantize {
                    settings: quantize.clone(),
                },
            ),
            (
                format!("perturb-{name}"),
                NativeOperation::Perturb { settings: perturb },
            ),
            (
                format!("separable-{name}"),
                NativeOperation::Separable {
                    settings: SeparableSettings { quantize, perturb },
                },
            ),
        ]);
    }
    recipes
}

/// Use the existing complete public methods without adding a public scheduling selector.
pub fn public(operation: &NativeOperation) -> PublicOperation {
    match operation {
        NativeOperation::Quantize { settings } => PublicOperation::Quantize {
            settings: settings.clone(),
        },
        NativeOperation::Perturb { settings } => PublicOperation::Perturb {
            settings: *settings,
        },
        NativeOperation::Separable { settings } => PublicOperation::Separable {
            settings: settings.clone(),
        },
        _ => unreachable!("S36 complete-call recipe"),
    }
}
