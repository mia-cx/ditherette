//! Effect extension contract and the ordered chain executor.
//!
//! The executor validates every step, checks the context each enabled step
//! needs, then applies enabled steps in array order. It knows nothing about
//! individual effects, so new effects never change sequencing.

use serde::{Deserialize, Serialize};

use crate::{
    image::contracts::PaletteEntry,
    prod::contract::{
        error::{DitheretteError, ErrorCode},
        request::{WorkingSpace, MAX_PALETTE_ENTRIES},
    },
};

/// Most steps one chain may hold, enabled or not.
pub const MAX_EFFECTS: usize = 64;

use super::{channel::Channel, image::EffectImage};

/// Shared inputs an effect may read. Ordinary effects read neither field.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EffectContext<'a> {
    /// Caller-ordered palette. Transparent entries carry no colour.
    pub palette: &'a [PaletteEntry],
    /// The working space final quantization matches in.
    pub space: Option<WorkingSpace>,
}

impl EffectContext<'_> {
    /// Visible colours among the first 256 entries, the palette quantization keeps.
    /// Caller order and duplicates are preserved.
    pub fn colors(&self) -> impl Iterator<Item = [u8; 3]> + '_ {
        let retained = &self.palette[..self.palette.len().min(MAX_PALETTE_ENTRIES)];
        retained.iter().filter_map(|entry| match entry {
            PaletteEntry::Color { rgb } => Some(*rgb),
            PaletteEntry::Transparent {} => None,
        })
    }
}

/// Context an effect requires before it can run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Needs {
    pub palette: bool,
    pub space: bool,
}

/// One colour operation. Implement this to add an effect.
pub trait Effect {
    /// Checks arguments only. `path` names this step, such as `effects.2`.
    fn validate(&self, path: &str) -> Result<(), DitheretteError>;

    /// Context this effect reads. The executor checks it before any pixel work.
    fn needs(&self) -> Needs {
        Needs::default()
    }

    /// Transforms RGB in place. Arguments and context are already validated.
    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>);

    /// `Some` when each selected channel maps independently of the others.
    /// Production tabulates such runs for byte input; the result must equal `apply`.
    fn channel_map(&self) -> Option<(Channel, &dyn ChannelFn)> {
        None
    }
}

/// The scalar map a per-channel effect applies to each selected channel.
pub trait ChannelFn {
    fn map(&self, value: f32) -> f32;
}

impl<E: Effect + ?Sized> Effect for Box<E> {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        (**self).validate(path)
    }

    fn needs(&self) -> Needs {
        (**self).needs()
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        (**self).apply(image, context)
    }

    fn channel_map(&self) -> Option<(Channel, &dyn ChannelFn)> {
        (**self).channel_map()
    }
}

/// One ordered chain entry. A disabled step keeps its arguments but does no work.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step<E> {
    pub enabled: bool,
    #[serde(flatten)]
    pub effect: E,
}

/// Validates every step and the context enabled steps need, without touching pixels.
pub fn validate_chain<E: Effect>(
    steps: &[Step<E>],
    context: &EffectContext<'_>,
) -> Result<(), DitheretteError> {
    if steps.len() > MAX_EFFECTS {
        return Err(DitheretteError::new(
            ErrorCode::InvalidSettings,
            "effects",
            format!("A chain holds at most {MAX_EFFECTS} effects."),
        ));
    }
    for (index, step) in steps.iter().enumerate() {
        step.effect.validate(&format!("effects.{index}"))?;
    }
    for (index, step) in steps.iter().enumerate().filter(|(_, step)| step.enabled) {
        let needs = step.effect.needs();
        if needs.palette && context.colors().next().is_none() {
            return Err(missing(
                index,
                "context.palette",
                "a visible palette colour",
            ));
        }
        if needs.space && context.space.is_none() {
            return Err(missing(index, "context.space", "a working space"));
        }
    }
    Ok(())
}

/// Validates, then applies each enabled step in order. Nothing is applied on error.
pub fn apply_chain<E: Effect>(
    image: &mut EffectImage,
    steps: &[Step<E>],
    context: &EffectContext<'_>,
) -> Result<(), DitheretteError> {
    validate_chain(steps, context)?;
    for step in steps.iter().filter(|step| step.enabled) {
        step.effect.apply(image, context);
    }
    Ok(())
}

/// Rejects NaN, infinities, and values outside `min..=max` with an argument path.
pub fn check_bounded(value: f32, min: f32, max: f32, path: String) -> Result<(), DitheretteError> {
    if value.is_finite() && (min..=max).contains(&value) {
        return Ok(());
    }
    Err(DitheretteError::new(
        ErrorCode::InvalidSettings,
        path,
        format!("Value must be finite and between {min} and {max}."),
    ))
}

fn missing(index: usize, path: &str, what: &str) -> DitheretteError {
    DitheretteError::new(
        ErrorCode::InvalidRequest,
        path,
        format!("Effect effects.{index} requires {what}."),
    )
}
