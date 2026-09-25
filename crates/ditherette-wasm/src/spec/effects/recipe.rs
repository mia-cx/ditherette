//! Built-in effect registry and its JSON recipe form.
//!
//! `BuiltinEffect` is the static registry: one tagged variant per built-in.
//! Decoding reports the failing step's index, so a typo never skips work.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::spec::contract::error::{DitheretteError, ErrorCode};

use super::{
    brightness_contrast::BrightnessContrast,
    chain::{Effect, EffectContext, Needs, Step},
    curves::Curves,
    exposure::Exposure,
    hue_saturation::HueSaturation,
    image::EffectImage,
    levels::Levels,
    white_balance::WhiteBalance,
};

/// Every effect the crate compiles in. The JSON tag is `effect`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "kebab-case")]
pub enum BuiltinEffect {
    Levels(Levels),
    Curves(Curves),
    BrightnessContrast(BrightnessContrast),
    Exposure(Exposure),
    WhiteBalance(WhiteBalance),
    HueSaturation(HueSaturation),
}

/// Dispatches one trait call to whichever built-in this is.
macro_rules! each {
    ($self:ident, $effect:ident => $call:expr) => {
        match $self {
            Self::Levels($effect) => $call,
            Self::Curves($effect) => $call,
            Self::BrightnessContrast($effect) => $call,
            Self::Exposure($effect) => $call,
            Self::WhiteBalance($effect) => $call,
            Self::HueSaturation($effect) => $call,
        }
    };
}

/// One serialized chain entry: the effect's tagged object plus `enabled`.
pub type EffectStep = Step<BuiltinEffect>;

impl Effect for BuiltinEffect {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        each!(self, effect => effect.validate(path))
    }

    fn needs(&self) -> Needs {
        each!(self, effect => effect.needs())
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        each!(self, effect => effect.apply(image, context))
    }
}

/// Decodes an ordered `effects` JSON array. Argument ranges are checked by validation.
pub fn decode_effects(json: &str) -> Result<Vec<EffectStep>, DitheretteError> {
    let value = serde_json::from_str(json).map_err(|error| malformed("effects", error))?;
    decode_steps(value, "effects")
}

/// Decodes each array element separately so errors name `effects.i`.
pub(super) fn decode_steps(value: Value, path: &str) -> Result<Vec<EffectStep>, DitheretteError> {
    let Value::Array(items) = value else {
        return Err(DitheretteError::new(
            ErrorCode::InvalidSettings,
            path,
            "Effects must be an array.",
        ));
    };
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            serde_json::from_value(item)
                .map_err(|error| malformed(&format!("{path}.{index}"), error))
        })
        .collect()
}

fn malformed(path: &str, error: serde_json::Error) -> DitheretteError {
    DitheretteError::new(ErrorCode::InvalidSettings, path, error.to_string())
}
