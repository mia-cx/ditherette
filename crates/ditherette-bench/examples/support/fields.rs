//! S26's fixed complete recipes, shared by declaration and untimed conformance.
use ditherette_bench::paired::{browser::PublicOperation, fields::*, quantize::*};

pub const SEED: u32 = 0x12345678;

pub fn adaptive(radius: u32) -> Placement {
    Placement::Adaptive {
        radius,
        threshold: 10.0,
        softness: 5.0,
    }
}

pub fn complete_recipes() -> Vec<(String, PublicOperation)> {
    let mut recipes = Vec::new();
    for (name, space, field, placement) in [
        (
            "srgb-bayer2-everywhere",
            WorkingSpace::Srgb,
            Field::Bayer {
                size: BayerSize::Two,
            },
            Placement::Everywhere {},
        ),
        (
            "linear-rgb-random-everywhere",
            WorkingSpace::LinearRgb,
            Field::Random { seed: SEED },
            Placement::Everywhere {},
        ),
        (
            "oklab-bayer4-adaptive1",
            WorkingSpace::Oklab,
            Field::Bayer {
                size: BayerSize::Four,
            },
            adaptive(1),
        ),
        (
            "oklch-random-adaptive2",
            WorkingSpace::Oklch,
            Field::Random { seed: SEED },
            adaptive(2),
        ),
        (
            "cielab-bayer8-everywhere",
            WorkingSpace::Cielab,
            Field::Bayer {
                size: BayerSize::Eight,
            },
            Placement::Everywhere {},
        ),
        (
            "cielch-random-adaptive1",
            WorkingSpace::Cielch,
            Field::Random { seed: SEED },
            adaptive(1),
        ),
        (
            "ycbcr-bayer16-everywhere",
            WorkingSpace::Ycbcr,
            Field::Bayer {
                size: BayerSize::Sixteen,
            },
            Placement::Everywhere {},
        ),
    ] {
        recipes.push((
            format!("perturb-{name}"),
            PublicOperation::Perturb {
                settings: PerturbPolicy {
                    space,
                    field,
                    placement,
                    strength: 0.7,
                },
            },
        ));
    }
    let mut palette: Vec<_> = (0..63)
        .map(|i| PaletteEntry::Color {
            rgb: [(i * 73) as u8, (i * 31 + 19) as u8, (i * 17 + 113) as u8],
        })
        .collect();
    palette.push(PaletteEntry::Transparent {});
    for (name, space, field, placement, matching) in [
        (
            "bayer4-oklab-everywhere-srgb-compuphase",
            WorkingSpace::Oklab,
            Field::Bayer {
                size: BayerSize::Four,
            },
            Placement::Everywhere {},
            MatchPolicy::SrgbCompuphase,
        ),
        (
            "random-ycbcr-adaptive2-oklch-hue-arc",
            WorkingSpace::Ycbcr,
            Field::Random { seed: SEED },
            adaptive(2),
            MatchPolicy::OklchHueArc,
        ),
    ] {
        recipes.push((
            format!("separable-{name}-palette64"),
            PublicOperation::Separable {
                settings: SeparableSettings {
                    perturb: PerturbPolicy {
                        space,
                        field,
                        placement,
                        strength: 0.7,
                    },
                    quantize: QuantizeSettings {
                        palette: palette.clone(),
                        alpha: AlphaPolicy::Preserve { threshold: 0.5 },
                        matching,
                    },
                },
            },
        ));
    }
    recipes
}
