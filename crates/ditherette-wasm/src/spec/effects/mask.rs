//! Step masks: curves whose output is a step's strength per pixel.
//!
//! A mask reads the pixel entering its step. Its curves multiply into one strength in `[0,1]`.

use serde::{Deserialize, Serialize};

use crate::spec::contract::error::{DitheretteError, ErrorCode};

use super::{
    curves::{
        validate_channel, validate_periodic_points, ChannelKind, ColourChannel, CurveGrid,
        CurveSpline, Curves, ResolvedChannel,
    },
    image::EffectImage,
};

/// Most curves one step's mask holds.
pub const MAX_MASK_CURVES: usize = 4;

/// Strength by one input channel. A hue input uses the cyclic spline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OneInputMask {
    pub x: ColourChannel,
    pub points: Vec<[f32; 2]>,
}

/// Strength by two input channels through a control grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TwoInputMask {
    pub x: ColourChannel,
    pub x2: ColourChannel,
    pub grid: CurveGrid,
}

/// A strict one-input or two-input mask curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MaskCurve {
    OneInput(OneInputMask),
    TwoInput(TwoInputMask),
}

/// Checks the count, then each curve, under `{step}.mask`.
pub fn validate(mask: &[MaskCurve], step: &str) -> Result<(), DitheretteError> {
    if mask.len() > MAX_MASK_CURVES {
        return Err(DitheretteError::new(
            ErrorCode::InvalidSettings,
            format!("{step}.mask"),
            format!("Expected 0 to {MAX_MASK_CURVES} mask curves."),
        ));
    }
    for (index, curve) in mask.iter().enumerate() {
        curve.validate(&format!("{step}.mask.{index}"))?;
    }
    Ok(())
}

impl MaskCurve {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        match self {
            Self::OneInput(curve) => {
                let x = validate_channel(curve.x, &format!("{path}.x"))?;
                let points = format!("{path}.points");
                Curves::validate_points(&curve.points, &points)?;
                if x.kind == ChannelKind::Hue {
                    validate_periodic_points(&curve.points, &points)?;
                }
                Ok(())
            }
            Self::TwoInput(curve) => {
                let x = validate_channel(curve.x, &format!("{path}.x"))?;
                let x2 = validate_channel(curve.x2, &format!("{path}.x2"))?;
                if curve.x2 == curve.x {
                    return Err(DitheretteError::new(
                        ErrorCode::InvalidSettings,
                        format!("{path}.x2"),
                        "A two-input curve must use two different input channels.",
                    ));
                }
                curve.grid.validate(
                    &format!("{path}.grid"),
                    x.kind == ChannelKind::Hue,
                    x2.kind == ChannelKind::Hue,
                )
            }
        }
    }
}

enum Prepared<'a> {
    OneInput {
        x: ResolvedChannel,
        spline: CurveSpline,
    },
    TwoInput {
        curve: &'a TwoInputMask,
        x: ResolvedChannel,
        x2: ResolvedChannel,
    },
}

impl Prepared<'_> {
    fn new(curve: &MaskCurve) -> Prepared<'_> {
        match curve {
            MaskCurve::OneInput(curve) => {
                let x = curve.x.resolve().expect("validated mask x channel");
                Prepared::OneInput {
                    x,
                    spline: CurveSpline::new(&curve.points, x.kind == ChannelKind::Hue),
                }
            }
            MaskCurve::TwoInput(curve) => Prepared::TwoInput {
                curve,
                x: curve.x.resolve().expect("validated mask x channel"),
                x2: curve.x2.resolve().expect("validated mask x2 channel"),
            },
        }
    }

    /// The curve's clamped value at `rgb`, faded toward 1 by the confidence of any hue input.
    fn value(&self, rgb: [f32; 3]) -> f32 {
        match self {
            Self::OneInput { x, spline } => {
                let coordinates = x.model.to_normalized(rgb);
                let value = spline.eval(coordinates[x.index]).clamp(0.0, 1.0);
                if x.kind != ChannelKind::Hue {
                    return value;
                }
                fade(value, x.model.hue_weight(rgb, coordinates))
            }
            Self::TwoInput { curve, x, x2 } => {
                let x_coordinates = x.model.to_normalized(rgb);
                let x2_coordinates = if x2.model == x.model {
                    x_coordinates
                } else {
                    x2.model.to_normalized(rgb)
                };
                let value = curve.grid.eval(
                    x_coordinates[x.index],
                    x2_coordinates[x2.index],
                    x.kind == ChannelKind::Hue,
                    x2.kind == ChannelKind::Hue,
                );
                let mut weight: f32 = 1.0;
                if x.kind == ChannelKind::Hue {
                    weight = x.model.hue_weight(rgb, x_coordinates);
                }
                if x2.kind == ChannelKind::Hue {
                    weight = weight.min(x2.model.hue_weight(rgb, x2_coordinates));
                }
                fade(value, weight)
            }
        }
    }
}

/// Near-grey hue is unreliable, so a hue-keyed value moves toward 1 as confidence falls.
fn fade(value: f32, weight: f32) -> f32 {
    if weight == 1.0 {
        value
    } else {
        1.0 - weight * (1.0 - value)
    }
}

/// Each pixel's strength: 1, times each curve's value in list order. `mask` is validated.
pub fn strengths(mask: &[MaskCurve], image: &EffectImage) -> Vec<f32> {
    let prepared: Vec<_> = mask.iter().map(Prepared::new).collect();
    image
        .rgb
        .iter()
        .map(|&rgb| {
            prepared
                .iter()
                .fold(1.0, |strength, curve| strength * curve.value(rgb))
        })
        .collect()
}

/// Moves `output` back toward `input` by `1 - strength`, per channel in carrier units.
/// Strength 1 returns `output` and strength 0 returns `input`, both exactly.
pub fn blend(input: [f32; 3], output: [f32; 3], strength: f32) -> [f32; 3] {
    if strength == 1.0 {
        return output;
    }
    if strength == 0.0 {
        return input;
    }
    std::array::from_fn(|channel| input[channel] + strength * (output[channel] - input[channel]))
}
