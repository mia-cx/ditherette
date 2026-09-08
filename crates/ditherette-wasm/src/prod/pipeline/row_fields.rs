//! Call-owned scheduling for direct quantization and RGBA8 field composition.

use super::execution::RowBandPolicy;
use crate::{
    image::ImageDimensions,
    prod::{contract::failure::Failure, dither::perturb, tiling::RowBandBuffers},
};

/// Field calls already charge one scalar converter in their method bookkeeping.
/// Bands add only the remaining worker converters and their assignment ownership.
pub(super) struct Plan {
    dimensions: ImageDimensions,
    policy: RowBandPolicy,
    field: bool,
    required: u64,
}

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
