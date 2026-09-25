//! Built-in effect registry and its JSON recipe form.
//!
//! `BuiltinEffect` is the static registry: one tagged variant per built-in.
//! Decoding reports the failing step's index, so a typo never skips work.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prod::contract::error::{DitheretteError, ErrorCode};

use super::{
    chain::{Effect, EffectContext, Needs, Step},
    image::EffectImage,
    levels::Levels,
};

/// Every effect the crate compiles in. The JSON tag is `effect`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "kebab-case")]
pub enum BuiltinEffect {
    Levels(Levels),
}

/// One serialized chain entry: the effect's tagged object plus `enabled`.
pub type EffectStep = Step<BuiltinEffect>;

impl Effect for BuiltinEffect {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        match self {
            Self::Levels(effect) => effect.validate(path),
        }
    }

    fn needs(&self) -> Needs {
        match self {
            Self::Levels(effect) => effect.needs(),
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        match self {
            Self::Levels(effect) => effect.apply(image, context),
        }
    }

    fn per_channel(&self) -> bool {
        match self {
            Self::Levels(effect) => effect.per_channel(),
        }
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        match self {
            Self::Levels(effect) => effect.map_channel(channel, value),
        }
    }
}

/// Decodes an ordered `effects` JSON array. Argument ranges are checked by validation.
pub fn decode_effects(json: &str) -> Result<Vec<EffectStep>, DitheretteError> {
    let value = serde_json::from_str(json).map_err(|error| malformed("effects", error))?;
    decode_steps(value, "effects")
}

/// Decodes each array element separately so errors name `effects.i`.
fn decode_steps(value: Value, path: &str) -> Result<Vec<EffectStep>, DitheretteError> {
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
