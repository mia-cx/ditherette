//! Palette fit: a palette-aware treatment written as ordinary curves.
//!
//! A `palette-fit` step either carries an explicit curve list (edited, locked) or analyses the
//! image reaching it against the context palette (`palette_fit_analysis`). The resolved list is
//! applied exactly as a `curves` step with the step's strength as each pixel's mask strength.

use serde::{Deserialize, Serialize};

use crate::spec::contract::{error::DitheretteError, request::WorkingSpace};

use super::{
    chain::{check_bounded, Effect, EffectContext, Needs},
    curves::{Curve, Curves},
    image::EffectImage,
    model::ColourModel,
    palette_fit_analysis::analyze,
};

/// Which analysis preset a `palette-fit` step uses. Only `fitted` exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FitLook {
    Fitted,
}

/// The space palette fit analyses and edits in: one of the lab/lch model pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FitSpace {
    Oklab,
    Cielab,
}

impl FitSpace {
    /// The opponent coordinates analysis reads, from `space.md`.
    pub fn working_space(self) -> WorkingSpace {
        match self {
            Self::Oklab => WorkingSpace::Oklab,
            Self::Cielab => WorkingSpace::Cielab,
        }
    }

    /// The lab model of the pair: lightness, `a`, and `b` channels.
    pub fn lab(self) -> ColourModel {
        match self {
            Self::Oklab => ColourModel::Oklab,
            Self::Cielab => ColourModel::Cielab,
        }
    }

    /// The cylindrical model of the pair: chroma and hue channels.
    pub fn lch(self) -> ColourModel {
        match self {
            Self::Oklab => ColourModel::Oklch,
            Self::Cielab => ColourModel::Cielch,
        }
    }

    /// Normalized units per opponent unit on the `a`/`b` axes, from `model.md`.
    /// Oklab divides by 0.8; cielab by 2.5 after its opponent scale of 100.
    pub fn opponent_scale(self) -> f32 {
        match self {
            Self::Oklab => 1.0 / 0.8,
            Self::Cielab => 100.0 / 250.0,
        }
    }
}

/// The `palette-fit` effect. `curves` of `null` analyses the image it receives;
/// a list applies as edited. `space` is the step's own; the context's is ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaletteFit {
    /// The look the analysis is tuned for.
    pub look: FitLook,
    /// The analysis space, and the lab/lch model pair the analysed curves use.
    pub space: FitSpace,
    /// Scales every curve's bend, in `[0, 1]`.
    pub strength: f32,
    /// `None` analyses automatically; a list is applied exactly like a `curves` step.
    pub curves: Option<Vec<Curve>>,
}

impl PaletteFit {
    /// The applied list: the step's own, or the analysis of the image it received.
    fn resolved(&self, image: &EffectImage, context: &EffectContext<'_>) -> Vec<Curve> {
        match &self.curves {
            Some(curves) => curves.clone(),
            None => analyze(image, context, self.space, self.look),
        }
    }
}

impl Effect for PaletteFit {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.strength, 0.0, 1.0, format!("{path}.strength"))?;
        match &self.curves {
            // An explicit list validates exactly like the `curves` effect's own.
            Some(curves) => Curves {
                curves: curves.clone(),
            }
            .validate(path),
            None => Ok(()),
        }
    }

    /// Only analysis reads the context, and it reads the palette alone.
    fn needs(&self) -> Needs {
        Needs {
            palette: self.curves.is_none(),
            space: false,
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        if self.strength == 0.0 {
            return;
        }
        let curves = self.resolved(image, context);
        // A step without a mask applies at `m = strength` on every pixel.
        let strength = vec![self.strength; image.rgb.len()];
        Curves { curves }.apply_masked(image, context, &strength);
    }

    /// The mask multiplies `strength` per pixel. Analysis still reads the whole unmasked input.
    fn apply_masked(&self, image: &mut EffectImage, context: &EffectContext<'_>, mask: &[f32]) {
        if self.strength == 0.0 {
            return;
        }
        let curves = self.resolved(image, context);
        let strengths: Vec<f32> = mask.iter().map(|&m| self.strength * m).collect();
        Curves { curves }.apply_masked(image, context, &strengths);
    }
}
