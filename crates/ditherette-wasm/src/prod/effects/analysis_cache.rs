//! Processor-owned memo of recolour analyses, keyed by exactly what analysis reads.
//!
//! The key hashes the sampled carrier values and alpha, the dimensions, the retained palette,
//! and the space. Any unchanged recolour stage therefore hits, whichever call reaches it, and
//! edits after it never re-analyse. New entries publish only when the call succeeds.

use std::{cell::RefCell, collections::TryReserveError, mem::size_of};

use sha2::{Digest, Sha256};

use super::{
    chain::EffectContext,
    curves::MAX_POINTS,
    image::EffectImage,
    recolour::{Group, RecolourRecipe, MAX_GROUPS},
    recolour_analysis::{analyze, sample_step},
};
use crate::prod::contract::request::WorkingSpace;

/// Published analyses retained, most recent last. Pending analyses per call are capped the same.
pub const CAPACITY: usize = 8;

type Key = [u8; 32];

#[derive(Debug, Default, PartialEq)]
struct Entries {
    published: Vec<(Key, RecolourRecipe)>,
    pending: Vec<(Key, RecolourRecipe)>,
}

/// Interior mutability lets effects reach the cache through a shared context.
#[derive(Debug, Default, PartialEq)]
pub struct AnalysisCache(RefCell<Entries>);

impl AnalysisCache {
    /// Upper bound on owned bytes: while settling, published holds up to twice its capacity
    /// before trimming, alongside a full pending list, each recipe at its largest.
    pub const fn capacity_bytes() -> u64 {
        let recipe = size_of::<(Key, RecolourRecipe)>()
            + MAX_POINTS * size_of::<[f32; 2]>()
            + MAX_GROUPS * size_of::<Group>();
        (3 * CAPACITY * recipe + size_of::<Self>()) as u64
    }

    /// The recipe `analyze` would derive for `image`, from the cache when its inputs repeat.
    pub fn analyze(
        &self,
        image: &EffectImage,
        context: &EffectContext<'_>,
    ) -> Result<RecolourRecipe, TryReserveError> {
        let key = key(image, context);
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

    /// Publishes this call's analyses on success and drops them on failure.
    pub fn settle(&mut self, success: bool) {
        let entries = self.0.get_mut();
        let pending = std::mem::take(&mut entries.pending);
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

/// SHA-256 of every input `analyze` reads, in its sampling order.
fn key(image: &EffectImage, context: &EffectContext<'_>) -> Key {
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
