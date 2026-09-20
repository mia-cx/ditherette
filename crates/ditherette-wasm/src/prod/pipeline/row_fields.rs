//! Call-owned scheduling for direct quantization and RGBA8 field composition.

use super::{execution::RowBandPolicy, quantize::QuantizeRequest};
use crate::{
    image::{contracts::PaletteEntry, ImageDimensions},
    prod::{
        contract::{
            failure::Failure,
            request::{
                AlphaPolicy, DitherPolicy, Field, MatchPolicy, PerturbPolicy, Placement,
                WorkingSpace,
            },
        },
        dither::perturb,
        tiling::{RowBandBuffers, WorkerBudget},
    },
};

/// Conservative cost classes use measured representative settings, not per-value predictions.
/// Small inputs, zero strength, unmeasured recipes, and one-worker pools stay scalar.
pub(super) fn measured_field(
    dimensions: ImageDimensions,
    policy: PerturbPolicy,
    workers: WorkerBudget,
) -> Option<RowBandPolicy> {
    if policy.strength == 0.0 {
        return None;
    }
    match (policy.space, policy.field, policy.placement) {
        (WorkingSpace::Srgb, Field::Random { .. }, Placement::Everywhere {}) => {
            srgb_policy(dimensions, workers)
        }
        (WorkingSpace::Oklab, Field::BlueNoise {}, Placement::Adaptive { radius: 2, .. })
            if dimensions.width() >= 65
                && dimensions.height() >= 49
                && workers.pool_size() >= 2 =>
        {
            Some(RowBandPolicy {
                height: 32,
                workers,
                active_workers: 2,
            })
        }
        _ => None,
    }
}

/// Separable scheduling includes its RGBA8 boundary and matching cost in the measured class.
pub(super) fn measured_indexed(
    dimensions: ImageDimensions,
    request: QuantizeRequest<'_>,
    dither: DitherPolicy,
    workers: WorkerBudget,
) -> Option<RowBandPolicy> {
    if !matches!(request.alpha, AlphaPolicy::Preserve { .. }) {
        return None;
    }
    let count = match request.matching {
        MatchPolicy::SrgbEuclidean => 16,
        MatchPolicy::OklabEuclidean => 64,
        _ => return None,
    };
    if request.palette.len() != count
        || !matches!(request.palette.last(), Some(PaletteEntry::Transparent {}))
        || !request.palette[..count - 1]
            .iter()
            .all(|entry| matches!(entry, PaletteEntry::Color { .. }))
    {
        return None;
    }
    match dither {
        DitherPolicy::None {} if request.matching == MatchPolicy::SrgbEuclidean => {
            srgb_policy(dimensions, workers)
        }
        DitherPolicy::Separable { perturb } if perturb.space == request.matching.space() => {
            measured_field(dimensions, perturb, workers)
        }
        _ => None,
    }
}

fn srgb_policy(dimensions: ImageDimensions, workers: WorkerBudget) -> Option<RowBandPolicy> {
    if dimensions.width() < 769 || dimensions.height() < 513 || workers.pool_size() < 2 {
        return None;
    }
    let (height, active_workers) = if workers.pool_size() >= 4 {
        (128, 4)
    } else {
        (32, 2)
    };
    Some(RowBandPolicy {
        height,
        workers,
        active_workers,
    })
}

/// Field calls already charge one scalar converter in their method bookkeeping.
/// Bands add only the remaining worker converters and their assignment ownership.
pub(super) struct Plan {
    dimensions: ImageDimensions,
    policy: RowBandPolicy,
    field: bool,
    required: u64,
}

#[cfg(test)]
mod tests;

impl Plan {
    pub(super) fn new(
        dimensions: ImageDimensions,
        policy: RowBandPolicy,
        field: bool,
    ) -> Result<Self, Failure> {
        let required = if field {
            perturb::required_band_capacity_bytes(
                dimensions,
                policy.height,
                policy.workers,
                policy.active_workers,
            )?
        } else {
            RowBandBuffers::<()>::required_bytes(
                dimensions,
                policy.height,
                policy.workers,
                policy.active_workers,
                &|_| Ok(0),
            )?
        };
        Ok(Self {
            dimensions,
            policy,
            field,
            required,
        })
    }

    pub(super) fn additional_capacity(&self) -> u64 {
        self.required
            - if self.field {
                super::perturb::working_capacity_bytes()
            } else {
                0
            }
    }

    /// The caller preflights this allocation with every other live call buffer first.
    pub(super) fn allocate(&self) -> Result<RowBandBuffers<()>, Failure> {
        if self.field {
            perturb::try_band_buffers(
                self.dimensions,
                self.policy.height,
                self.policy.workers,
                self.policy.active_workers,
                self.required,
            )
        } else {
            RowBandBuffers::try_new(
                self.dimensions,
                self.policy.height,
                self.policy.workers,
                self.policy.active_workers,
                self.required,
                &|_| Ok(0),
            )
        }
    }
}
