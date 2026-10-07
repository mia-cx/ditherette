//! Palette fit: a curve list, explicit or analysed, applied like the `curves` effect.
//!
//! Mirrors `spec::effects::palette_fit`. A step with `curves: null` analyses the image reaching it
//! (`palette_fit_analysis`); resolution fills the list first, so the applied path is pointwise.

use serde::{Deserialize, Serialize};

use crate::prod::contract::{error::DitheretteError, request::WorkingSpace};

use super::{
    chain::{check_bounded, Effect, EffectContext, Needs, PreparedPointwise, StackPath},
    curves::{try_clone_curves, Curve, Curves, PreparedCurves},
    image::EffectImage,
    model::ColourModel,
    palette_fit_analysis::{analyze, ANALYSIS_BYTES},
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
    /// The opponent coordinates analysis reads.
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

    /// Normalized units per opponent unit on the `a`/`b` axes.
    /// Oklab divides by 0.8; cielab by 2.5 after its opponent scale of 100.
    pub fn opponent_scale(self) -> f32 {
        match self {
            Self::Oklab => 1.0 / 0.8,
            Self::Cielab => 100.0 / 250.0,
        }
    }
}

/// The `palette-fit` effect: analysis or an explicit list, each like `curves` once resolved.
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
            Some(curves) => {
                try_clone_curves(curves).expect("resolved curves fit the charged working memory")
            }
            None => self.analyze(image, context),
        }
    }

    /// Analysis of the whole unmasked image, cached where the context carries a cache.
    fn analyze(&self, image: &EffectImage, context: &EffectContext<'_>) -> Vec<Curve> {
        match context.analyses {
            Some(cache) => cache.analyze_palette_fit(image, context, self.space, self.look),
            None => analyze(image, context, self.space, self.look),
        }
        .expect("analysis samples fit the charged working memory")
    }
}

impl Effect for PaletteFit {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        check_bounded(self.strength, 0.0, 1.0, format_args!("{path}.strength"))?;
        match &self.curves {
            // An explicit list validates exactly like the `curves` effect's own.
            Some(curves) => {
                Curves::validate_list(curves, StackPath::new(format_args!("{path}")).as_str())
            }
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

    /// Analysis samples and the emitted list are charged once per unresolved step.
    fn working_bytes(&self) -> u64 {
        if self.curves.is_none() {
            ANALYSIS_BYTES
        } else {
            0
        }
    }

    /// Pointwise once its list is known. Resolution supplies one before this runs.
    fn pointwise(&self) -> bool {
        self.strength == 0.0 || self.curves.is_some()
    }

    fn prepare_pointwise(&self) -> PreparedPointwise {
        match &self.curves {
            Some(curves) => PreparedPointwise::Curves(PreparedCurves::new(curves)),
            None => PreparedPointwise::Direct,
        }
    }

    /// The list at bend strength `strength`, exactly `curves` at that mask strength.
    fn map_prepared(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        _context: &EffectContext<'_>,
    ) -> [f32; 3] {
        match (prepared, &self.curves) {
            (PreparedPointwise::Curves(prepared), Some(curves)) => {
                prepared.map(curves, rgb, self.strength)
            }
            _ => rgb,
        }
    }

    /// The mask multiplies `strength` for this pixel; zero leaves it unchanged.
    fn map_prepared_masked(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        strength: f32,
        _context: &EffectContext<'_>,
    ) -> [f32; 3] {
        let strength = self.strength * strength;
        if strength == 0.0 {
            return rgb;
        }
        match (prepared, &self.curves) {
            (PreparedPointwise::Curves(prepared), Some(curves)) => {
                prepared.map(curves, rgb, strength)
            }
            _ => rgb,
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        if self.strength == 0.0 {
            return;
        }
        let curves = self.resolved(image, context);
        let prepared = PreparedCurves::new(&curves);
        for rgb in &mut image.rgb {
            *rgb = prepared.map(&curves, *rgb, self.strength);
        }
    }

    /// Analysis still reads the whole unmasked input; the mask multiplies `strength` per pixel.
    fn apply_masked(
        &self,
        image: &mut EffectImage,
        _input: &[[f32; 3]],
        strengths: &[f32],
        context: &EffectContext<'_>,
    ) {
        if self.strength == 0.0 {
            return;
        }
        let curves = self.resolved(image, context);
        let prepared = PreparedCurves::new(&curves);
        for (rgb, &mask) in image.rgb.iter_mut().zip(strengths) {
            let strength = self.strength * mask;
            if strength != 0.0 {
                *rgb = prepared.map(&curves, *rgb, strength);
            }
        }
    }
}
