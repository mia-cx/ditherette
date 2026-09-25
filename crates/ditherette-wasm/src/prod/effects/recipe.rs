//! Built-in effect registry and its JSON recipe form.
//!
//! `BuiltinEffect` is the static registry: one tagged variant per built-in.
//! Decoding reports the failing step's index, so a typo never skips work.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::TryReserveError;

use crate::{
    image::ImageDimensions,
    prod::contract::error::{DitheretteError, ErrorCode},
};

use super::{
    brightness_contrast::BrightnessContrast,
    chain::{Effect, EffectContext, Needs, Step},
    curves::Curves,
    exposure::Exposure,
    hue_saturation::HueSaturation,
    image::EffectImage,
    levels::Levels,
    table::ChannelTables,
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

/// One serialized chain entry: the effect's tagged object plus `enabled`.
pub type EffectStep = Step<BuiltinEffect>;

impl Effect for BuiltinEffect {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        match self {
            Self::Levels(effect) => effect.validate(path),
            Self::Curves(effect) => effect.validate(path),
            Self::BrightnessContrast(effect) => effect.validate(path),
            Self::Exposure(effect) => effect.validate(path),
            Self::WhiteBalance(effect) => effect.validate(path),
            Self::HueSaturation(effect) => effect.validate(path),
        }
    }

    fn needs(&self) -> Needs {
        match self {
            Self::Levels(effect) => effect.needs(),
            Self::Curves(effect) => effect.needs(),
            Self::BrightnessContrast(effect) => effect.needs(),
            Self::Exposure(effect) => effect.needs(),
            Self::WhiteBalance(effect) => effect.needs(),
            Self::HueSaturation(effect) => effect.needs(),
        }
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        match self {
            Self::Levels(effect) => effect.apply(image, context),
            Self::Curves(effect) => effect.apply(image, context),
            Self::BrightnessContrast(effect) => effect.apply(image, context),
            Self::Exposure(effect) => effect.apply(image, context),
            Self::WhiteBalance(effect) => effect.apply(image, context),
            Self::HueSaturation(effect) => effect.apply(image, context),
        }
    }

    fn per_channel(&self) -> bool {
        match self {
            Self::Levels(effect) => effect.per_channel(),
            Self::Curves(effect) => effect.per_channel(),
            Self::BrightnessContrast(effect) => effect.per_channel(),
            Self::Exposure(effect) => effect.per_channel(),
            Self::WhiteBalance(effect) => effect.per_channel(),
            Self::HueSaturation(effect) => effect.per_channel(),
        }
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        match self {
            Self::Levels(effect) => effect.map_channel(channel, value),
            Self::Curves(effect) => effect.map_channel(channel, value),
            Self::BrightnessContrast(effect) => effect.map_channel(channel, value),
            Self::Exposure(effect) => effect.map_channel(channel, value),
            Self::WhiteBalance(effect) => effect.map_channel(channel, value),
            Self::HueSaturation(effect) => effect.map_channel(channel, value),
        }
    }

    fn apply_tabulated(
        &self,
        data: &[u8],
        dimensions: ImageDimensions,
        tables: &ChannelTables,
        context: &EffectContext<'_>,
    ) -> Option<Result<EffectImage, TryReserveError>> {
        match self {
            Self::Levels(effect) => effect.apply_tabulated(data, dimensions, tables, context),
            Self::Curves(effect) => effect.apply_tabulated(data, dimensions, tables, context),
            Self::BrightnessContrast(effect) => {
                effect.apply_tabulated(data, dimensions, tables, context)
            }
            Self::Exposure(effect) => effect.apply_tabulated(data, dimensions, tables, context),
            Self::WhiteBalance(effect) => effect.apply_tabulated(data, dimensions, tables, context),
            Self::HueSaturation(effect) => {
                effect.apply_tabulated(data, dimensions, tables, context)
            }
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
