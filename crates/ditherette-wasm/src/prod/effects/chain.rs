//! Effect extension contract and the ordered chain executor.
//!
//! The executor validates every step, checks the context each enabled step
//! needs, then applies enabled steps in array order. It knows nothing about
//! individual effects, so new effects never change sequencing.

use std::{fmt, mem::size_of};

use serde::{Deserialize, Serialize};

use crate::{
    image::contracts::PaletteEntry,
    prod::contract::{
        error::{DitheretteError, ErrorCode},
        request::{WorkingSpace, MAX_PALETTE_ENTRIES},
    },
};

use super::{
    analysis_cache::AnalysisCache,
    curves::PreparedCurves,
    image::{EffectImage, CARRIER_LIMIT},
    mask::{self, blend, MaskCurve, PreparedMask},
    table::ChannelTables,
};

/// Most steps one chain may hold, enabled or not.
pub const MAX_EFFECTS: usize = 64;

/// Per-call state for one pointwise effect. Each variant keeps its pixel work allocation-free.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PreparedPointwise {
    #[default]
    Direct,
    HueSaturation {
        sin: f32,
        cos: f32,
        scale: f32,
    },
    Curves(PreparedCurves),
}

impl PreparedPointwise {
    fn reads_linear_input(&self) -> bool {
        matches!(self, Self::HueSaturation { .. })
    }
}

/// An enabled step as the planner sees it: the effect and its mask curves.
pub struct Planned<'a, E> {
    pub effect: &'a E,
    pub mask: &'a [MaskCurve],
}

impl<E> Clone for Planned<'_, E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for Planned<'_, E> {}

/// One step's prepared map and, when it is masked, its prepared mask.
struct PreparedStep {
    map: PreparedPointwise,
    mask: Option<PreparedMask>,
}

/// Prepared maps for one pointwise run, plus an exact linear table when its first map can use it.
pub struct PreparedPointwiseState<'steps, 'tables, E> {
    steps: &'steps [Planned<'steps, E>],
    prepared: Vec<PreparedStep>,
    tables: &'tables ChannelTables,
    linear: Option<[[f32; 256]; 3]>,
}

/// Bytes `PreparedPointwiseState` reserves per step.
pub const PREPARED_STEP_BYTES: usize = size_of::<PreparedStep>();

impl<'steps, 'tables, E: Effect> PreparedPointwiseState<'steps, 'tables, E> {
    pub fn try_new(
        steps: &'steps [Planned<'steps, E>],
        tables: &'tables ChannelTables,
    ) -> Result<Self, std::collections::TryReserveError> {
        debug_assert!(steps.len() <= MAX_EFFECTS);
        debug_assert!(steps.iter().all(|step| step.effect.pointwise()));
        let mut prepared = Vec::new();
        prepared.try_reserve_exact(steps.len())?;
        for step in steps {
            let mask = PreparedMask::new(step.mask);
            prepared.push(PreparedStep {
                map: step.effect.prepare_pointwise(),
                mask: (!mask.is_full()).then_some(mask),
            });
        }
        let linear = prepared
            .first()
            .filter(|first| first.mask.is_none() && first.map.reads_linear_input())
            .map(|_| tables.linear());
        Ok(Self {
            steps,
            prepared,
            tables,
            linear,
        })
    }

    /// Maps one original byte colour and preserves the chain's clamp after every effect.
    /// A masked step reads its strength from the colour entering it.
    pub fn map(&self, bytes: [u8; 3], context: &EffectContext<'_>) -> [f32; 3] {
        let mut start = 0;
        let mut rgb = match self.linear.as_ref() {
            Some(linear) => {
                start = 1;
                let input = std::array::from_fn(|channel| linear[channel][bytes[channel] as usize]);
                self.steps[0]
                    .effect
                    .map_prepared_linear(&self.prepared[0].map, input, context)
                    .map(|value| value.clamp(-CARRIER_LIMIT, CARRIER_LIMIT))
            }
            None => std::array::from_fn(|channel| self.tables.unit(channel, bytes[channel])),
        };
        for (step, prepared) in self.steps[start..]
            .iter()
            .zip(&self.prepared[start..self.steps.len()])
        {
            rgb = match &prepared.mask {
                None => step.effect.map_prepared(&prepared.map, rgb, context),
                Some(mask) => {
                    let strength = mask.strength(step.mask, rgb);
                    step.effect
                        .map_prepared_masked(&prepared.map, rgb, strength, context)
                }
            }
            .map(|value| value.clamp(-CARRIER_LIMIT, CARRIER_LIMIT));
        }
        rgb
    }
}

/// Shared inputs an effect may read. Ordinary effects read neither field.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EffectContext<'a> {
    /// Caller-ordered palette. Transparent entries carry no colour.
    pub palette: &'a [PaletteEntry],
    /// The working space final quantization matches in.
    pub space: Option<WorkingSpace>,
    /// Production-only memo for recipe-less recolour steps. `None` analyses every time.
    pub analyses: Option<&'a AnalysisCache>,
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

/// A request path formatted on the stack. Built-in paths stay far below its 64 bytes;
/// anything longer is cut short rather than allocated.
pub struct StackPath {
    bytes: [u8; 64],
    len: usize,
}

impl StackPath {
    pub fn new(args: fmt::Arguments<'_>) -> Self {
        let mut path = Self {
            bytes: [0; 64],
            len: 0,
        };
        // Overflow only truncates; paths are ASCII, so the cut stays on a character boundary.
        let _ = fmt::write(&mut path, args);
        path
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or("effects")
    }
}

impl fmt::Write for StackPath {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let room = self.bytes.len() - self.len;
        let taken = text.len().min(room);
        self.bytes[self.len..self.len + taken].copy_from_slice(&text.as_bytes()[..taken]);
        self.len += taken;
        if taken < text.len() {
            return Err(fmt::Error);
        }
        Ok(())
    }
}

/// One colour operation. Implement this to add an effect.
pub trait Effect {
    /// Checks arguments only. `path` names this step, such as `effects.2`.
    fn validate(&self, path: &str) -> Result<(), DitheretteError>;

    /// Context this effect reads. The executor checks it before any pixel work.
    fn needs(&self) -> Needs {
        Needs::default()
    }

    /// Checks arguments against a context that already meets `needs`, for an enabled step.
    fn check_context(
        &self,
        _context: &EffectContext<'_>,
        _path: &str,
    ) -> Result<(), DitheretteError> {
        Ok(())
    }

    /// Transforms RGB in place. Arguments and context are already validated.
    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>);

    /// Applies with per-pixel `strengths` in `[0,1]`. `input` is the image's RGB on entry,
    /// which the caller copied. By default the output moves back toward it in RGB.
    fn apply_masked(
        &self,
        image: &mut EffectImage,
        input: &[[f32; 3]],
        strengths: &[f32],
        context: &EffectContext<'_>,
    ) {
        self.apply(image, context);
        for ((rgb, &input), &strength) in image.rgb.iter_mut().zip(input).zip(strengths) {
            *rgb = blend(input, *rgb, strength);
        }
    }

    /// One pixel's masked map with state from `prepare_pointwise`. It must equal
    /// `apply_masked` on that pixel at that strength.
    fn map_prepared_masked(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        strength: f32,
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        if strength == 0.0 {
            return rgb;
        }
        blend(rgb, self.map_prepared(prepared, rgb, context), strength)
    }

    /// True when each output channel depends only on the same input channel.
    /// Production tabulates runs of such effects for byte input.
    fn per_channel(&self) -> bool {
        false
    }

    /// One channel's map (0 red, 1 green, 2 blue). Called only when `per_channel` is true;
    /// it must equal what `apply` does to that channel.
    fn map_channel(&self, _channel: usize, value: f32) -> f32 {
        value
    }

    /// Maps one channel with state from `prepare_pointwise` while tables are built.
    fn map_prepared_channel(
        &self,
        _prepared: &PreparedPointwise,
        channel: usize,
        value: f32,
    ) -> f32 {
        self.map_channel(channel, value)
    }

    /// Scratch bytes `apply` allocates beyond the carrier and memo, for memory accounting.
    fn working_bytes(&self) -> u64 {
        0
    }

    /// True when each output pixel depends only on the same input pixel.
    /// Production memoizes chains of such effects by input colour.
    fn pointwise(&self) -> bool {
        self.per_channel()
    }

    /// One pixel's map. Called only when `pointwise` is true; it must equal `apply` on that pixel.
    fn map_pixel(&self, rgb: [f32; 3], _context: &EffectContext<'_>) -> [f32; 3] {
        std::array::from_fn(|channel| self.map_channel(channel, rgb[channel]))
    }

    /// Builds fixed-size state reused by every memo miss in one call.
    fn prepare_pointwise(&self) -> PreparedPointwise {
        PreparedPointwise::Direct
    }

    /// Maps a pixel with state from `prepare_pointwise`.
    fn map_prepared(
        &self,
        _prepared: &PreparedPointwise,
        rgb: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        self.map_pixel(rgb, context)
    }

    /// Maps a pre-decoded linear input. Only prepared variants that request it call this method.
    fn map_prepared_linear(
        &self,
        _prepared: &PreparedPointwise,
        _linear: [f32; 3],
        _context: &EffectContext<'_>,
    ) -> [f32; 3] {
        unreachable!("this effect did not prepare a linear-input map")
    }
}

impl<E: Effect + ?Sized> Effect for Box<E> {
    fn validate(&self, path: &str) -> Result<(), DitheretteError> {
        (**self).validate(path)
    }

    fn needs(&self) -> Needs {
        (**self).needs()
    }

    fn check_context(
        &self,
        context: &EffectContext<'_>,
        path: &str,
    ) -> Result<(), DitheretteError> {
        (**self).check_context(context, path)
    }

    fn apply(&self, image: &mut EffectImage, context: &EffectContext<'_>) {
        (**self).apply(image, context)
    }

    fn apply_masked(
        &self,
        image: &mut EffectImage,
        input: &[[f32; 3]],
        strengths: &[f32],
        context: &EffectContext<'_>,
    ) {
        (**self).apply_masked(image, input, strengths, context)
    }

    fn map_prepared_masked(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        strength: f32,
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        (**self).map_prepared_masked(prepared, rgb, strength, context)
    }

    fn per_channel(&self) -> bool {
        (**self).per_channel()
    }

    fn map_channel(&self, channel: usize, value: f32) -> f32 {
        (**self).map_channel(channel, value)
    }

    fn map_prepared_channel(
        &self,
        prepared: &PreparedPointwise,
        channel: usize,
        value: f32,
    ) -> f32 {
        (**self).map_prepared_channel(prepared, channel, value)
    }

    fn working_bytes(&self) -> u64 {
        (**self).working_bytes()
    }

    fn pointwise(&self) -> bool {
        (**self).pointwise()
    }

    fn map_pixel(&self, rgb: [f32; 3], context: &EffectContext<'_>) -> [f32; 3] {
        (**self).map_pixel(rgb, context)
    }

    fn prepare_pointwise(&self) -> PreparedPointwise {
        (**self).prepare_pointwise()
    }

    fn map_prepared(
        &self,
        prepared: &PreparedPointwise,
        rgb: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        (**self).map_prepared(prepared, rgb, context)
    }

    fn map_prepared_linear(
        &self,
        prepared: &PreparedPointwise,
        linear: [f32; 3],
        context: &EffectContext<'_>,
    ) -> [f32; 3] {
        (**self).map_prepared_linear(prepared, linear, context)
    }
}

/// One ordered chain entry. A disabled step keeps its arguments but does no work.
/// An empty mask means full strength everywhere, and is left out of the JSON.
/// Production chains that substitute effects borrow the caller's mask as `M = &[MaskCurve]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(
    serialize = "E: Serialize, M: Serialize + AsRef<[MaskCurve]>",
    deserialize = "E: Deserialize<'de>, M: Deserialize<'de> + Default"
))]
pub struct Step<E, M = Vec<MaskCurve>> {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "is_unmasked")]
    pub mask: M,
    #[serde(flatten)]
    pub effect: E,
}

fn is_unmasked<M: AsRef<[MaskCurve]>>(mask: &M) -> bool {
    mask.as_ref().is_empty()
}

impl<E, M: AsRef<[MaskCurve]>> Step<E, M> {
    /// The effect and mask the planner runs.
    pub fn planned(&self) -> Planned<'_, E> {
        Planned {
            effect: &self.effect,
            mask: self.mask.as_ref(),
        }
    }
}

/// Validates every step and the context enabled steps need, without touching pixels.
pub fn validate_chain<E: Effect, M: AsRef<[MaskCurve]>>(
    steps: &[Step<E, M>],
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
        let path = StackPath::new(format_args!("effects.{index}"));
        step.effect.validate(path.as_str())?;
        mask::validate(step.mask.as_ref(), path.as_str())?;
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
        step.effect.check_context(
            context,
            StackPath::new(format_args!("effects.{index}")).as_str(),
        )?;
    }
    Ok(())
}

/// Validates, then applies each enabled step in order. Nothing is applied on error.
pub fn apply_chain<E: Effect, M: AsRef<[MaskCurve]>>(
    image: &mut EffectImage,
    steps: &[Step<E, M>],
    context: &EffectContext<'_>,
) -> Result<(), DitheretteError> {
    validate_chain(steps, context)?;
    for step in steps.iter().filter(|step| step.enabled) {
        let mask = step.mask.as_ref();
        if mask.is_empty() {
            step.effect.apply(image, context);
        } else {
            let input = image.rgb.clone();
            let strengths = mask::strengths(mask, &input);
            step.effect.apply_masked(image, &input, &strengths, context);
        }
        image.bound();
    }
    Ok(())
}

/// Rejects NaN, infinities, and values outside `min..=max` with an argument path.
/// Production formats `path` only on failure, so validating a valid chain never allocates.
pub fn check_bounded(
    value: f32,
    min: f32,
    max: f32,
    path: fmt::Arguments<'_>,
) -> Result<(), DitheretteError> {
    if value.is_finite() && (min..=max).contains(&value) {
        return Ok(());
    }
    Err(DitheretteError::new(
        ErrorCode::InvalidSettings,
        path.to_string(),
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
