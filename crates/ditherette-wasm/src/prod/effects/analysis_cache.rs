//! Processor-owned memo of recolour and palette-fit analyses, keyed by exactly what they read.
//!
//! The key hashes the sampled carrier values and alpha, the dimensions, the retained palette,
//! and the space (and look, for palette fit). Any unchanged analysis stage therefore hits,
//! whichever call reaches it, and edits after it never re-analyse. New entries publish only when
//! the call succeeds.

use std::{cell::RefCell, collections::TryReserveError, mem::size_of};

use sha2::{Digest, Sha256};

use super::{
    chain::EffectContext,
    curves::{Curve, MAX_POINTS},
    image::EffectImage,
    palette_fit::{FitLook, FitSpace},
    palette_fit_analysis,
    recolour::{Group, RecolourRecipe, MAX_GROUPS},
    recolour_analysis::{analyze, sample_step},
};
use crate::prod::contract::request::WorkingSpace;

/// Published analyses retained, most recent last. Pending analyses per call are capped the same.
pub const CAPACITY: usize = 8;
/// Palette-fit curves deduplicated within a call. Fit entries are call-scoped working memory,
/// not retained like recipes, so the instance's planned capacity stays at its fixed floor.
pub const FIT_CAPACITY: usize = 3;

type Key = [u8; 32];

#[derive(Debug, Default, PartialEq)]
struct Entries {
    published: Vec<(Key, RecolourRecipe)>,
    pending: Vec<(Key, RecolourRecipe)>,
    fit_pending: Vec<(Key, Vec<Curve>)>,
}

/// Interior mutability lets effects reach the cache through a shared context.
#[derive(Debug, Default, PartialEq)]
pub struct AnalysisCache(RefCell<Entries>);

impl AnalysisCache {
    /// Upper bound on owned bytes: while settling, each published list holds up to twice its
    /// capacity before trimming, alongside a full pending list, each entry at its largest.
    /// `fit_pending` is call-scoped and counted by `fit_working_bytes` instead.
    pub const fn capacity_bytes() -> u64 {
        let recipe = size_of::<(Key, RecolourRecipe)>()
            + MAX_POINTS * size_of::<[f32; 2]>()
            + MAX_GROUPS * size_of::<Group>();
        (3 * CAPACITY * recipe + size_of::<Self>()) as u64
    }

    /// The bound calls carrying this cache charge as working capacity: the call-scoped
    /// `fit_pending` list at its largest. Emitted lists hold at most 5 curves: a 13-point
    /// turn, a 7-point tone, two 2-point shifts, and a 12-by-5 gain grid — at most 640
    /// bytes of points and grid data.
    pub const fn fit_working_bytes() -> u64 {
        let fit = size_of::<(Key, Vec<Curve>)>() + 5 * size_of::<Curve>() + 640;
        (FIT_CAPACITY * fit) as u64
    }

    /// The recipe `analyze` would derive for `image`, from the cache when its inputs repeat.
    pub fn analyze(
        &self,
        image: &EffectImage,
        context: &EffectContext<'_>,
    ) -> Result<RecolourRecipe, TryReserveError> {
        let key = recolour_key(image, context);
        {
            let mut entries = self.0.borrow_mut();
            if let Some(index) = entries.published.iter().position(|(k, _)| *k == key) {
                // Moving the hit to the end reuses the list's own capacity.
                let entry = entries.published.remove(index);
                let recipe = entry.1.try_clone();
                entries.published.push(entry);
                return recipe;
            }
            if let Some((_, recipe)) = entries.pending.iter().find(|(k, _)| *k == key) {
                return recipe.try_clone();
            }
        }
        let recipe = analyze(image, context)?;
        let mut entries = self.0.borrow_mut();
        // Caching is optional: if either reservation fails, the call still has its recipe.
        if entries.pending.len() < CAPACITY && entries.pending.try_reserve(1).is_ok() {
            if let Ok(copy) = recipe.try_clone() {
                entries.pending.push((key, copy));
            }
        }
        Ok(recipe)
    }

    /// The curves a `palette-fit` step would derive for `image`, cached by its inputs:
    /// the image, palette, and the step's `space` and `look`.
    pub fn analyze_palette_fit(
        &self,
        image: &EffectImage,
        context: &EffectContext<'_>,
        space: FitSpace,
        look: FitLook,
    ) -> Result<Vec<Curve>, TryReserveError> {
        let key = palette_fit_key(image, context, space, look);
        {
            let entries = self.0.borrow();
            if let Some((_, curves)) = entries.fit_pending.iter().find(|(k, _)| *k == key) {
                return super::curves::try_clone_curves(curves);
            }
        }
        let curves = palette_fit_analysis::analyze(image, context, space, look)?;
        let mut entries = self.0.borrow_mut();
        // Caching is optional: if either reservation fails, the call still has its curves.
        if entries.fit_pending.len() < FIT_CAPACITY && entries.fit_pending.try_reserve(1).is_ok() {
            if let Ok(copy) = super::curves::try_clone_curves(&curves) {
                entries.fit_pending.push((key, copy));
            }
        }
        Ok(curves)
    }

    /// Publishes this call's analyses on success and drops them on failure.
    pub fn settle(&mut self, success: bool) {
        let entries = self.0.get_mut();
        let pending = std::mem::take(&mut entries.pending);
        // Call-scoped fit curves are always dropped.
        entries.fit_pending.clear();
        // A failed reservation only drops these optional entries.
        if !success || entries.published.try_reserve(pending.len()).is_err() {
            return;
        }
        for entry in pending {
            if !entries.published.iter().any(|(k, _)| *k == entry.0) {
                entries.published.push(entry);
            }
        }
        let excess = entries.published.len().saturating_sub(CAPACITY);
        entries.published.drain(..excess);
    }

    /// Published entry count, for tests and diagnostics.
    pub fn len(&self) -> usize {
        self.0.borrow().published.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// SHA-256 of the shared inputs an analysis reads, in its sampling order.
fn base_key(image: &EffectImage, context: &EffectContext<'_>) -> Sha256 {
    let mut hash = Sha256::new();
    let width = image.dimensions.width_usize();
    let height = image.dimensions.height() as usize;
    hash.update((width as u64).to_le_bytes());
    hash.update((height as u64).to_le_bytes());
    // Length first, so the palette and sample records cannot shift into each other.
    hash.update((context.colors().count() as u64).to_le_bytes());
    for color in context.colors() {
        hash.update(color);
    }
    let step = sample_step(width, height);
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let index = y * width + x;
            let alpha = image.alpha[index];
            hash.update([alpha]);
            if alpha > 0 {
                for channel in image.rgb[index] {
                    hash.update(channel.to_bits().to_le_bytes());
                }
            }
        }
    }
    hash
}

/// SHA-256 of every input `analyze` reads: the context's working space included.
fn recolour_key(image: &EffectImage, context: &EffectContext<'_>) -> Key {
    let mut hash = Sha256::new();
    hash.update(b"ditherette-recolour-analysis-v1\0");
    let width = image.dimensions.width_usize();
    let height = image.dimensions.height() as usize;
    hash.update((width as u64).to_le_bytes());
    hash.update((height as u64).to_le_bytes());
    let space = context.space.map_or(u8::MAX, space_tag);
    hash.update([space]);
    // Length first, so the palette and sample records cannot shift into each other.
    hash.update((context.colors().count() as u64).to_le_bytes());
    for color in context.colors() {
        hash.update(color);
    }
    let step = sample_step(width, height);
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let index = y * width + x;
            let alpha = image.alpha[index];
            hash.update([alpha]);
            if alpha > 0 {
                for channel in image.rgb[index] {
                    hash.update(channel.to_bits().to_le_bytes());
                }
            }
        }
    }
    hash.finalize().into()
}

/// SHA-256 of every input palette-fit analysis reads: the step's own `space` and `look`,
/// not the context's space, which the step ignores.
fn palette_fit_key(
    image: &EffectImage,
    context: &EffectContext<'_>,
    space: FitSpace,
    look: FitLook,
) -> Key {
    let mut hash = Sha256::new();
    hash.update(b"ditherette-palette-fit-analysis-v1\0");
    hash.update([match space {
        FitSpace::Oklab => 0,
        FitSpace::Cielab => 1,
    }]);
    hash.update([match look {
        FitLook::Fitted => 0,
        FitLook::Natural => 1,
        FitLook::Vivid => 2,
    }]);
    hash.update(base_key(image, context).finalize());
    hash.finalize().into()
}

fn space_tag(space: WorkingSpace) -> u8 {
    match space {
        WorkingSpace::Srgb => 0,
        WorkingSpace::LinearRgb => 1,
        WorkingSpace::Oklab => 2,
        WorkingSpace::Oklch => 3,
        WorkingSpace::Cielab => 4,
        WorkingSpace::Cielch => 5,
        WorkingSpace::Ycbcr => 6,
    }
}
