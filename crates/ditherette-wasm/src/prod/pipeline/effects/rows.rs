//! Explicit pointwise row workers. Resolve image-wide recipes once before entering.

use crate::{
    image::ImageDimensions,
    prod::{
        contract::failure::Failure,
        effects::{self, operation::ResolvedStep, EffectContext, EffectStep},
        pipeline::execution::RowBandPolicy,
        tiling::{bands_for_output_height, RowBandBuffers},
    },
};

pub(super) struct Rows {
    dimensions: ImageDimensions,
    policy: RowBandPolicy,
    metadata: u64,
    capacity: u64,
}

impl Rows {
    pub(super) fn new(
        dimensions: ImageDimensions,
        policy: Option<RowBandPolicy>,
        effects: &[EffectStep],
    ) -> Result<Option<Self>, Failure> {
        let Some(policy) = policy else {
            return Ok(None);
        };
        let metadata = RowBandBuffers::<()>::required_bytes(
            dimensions,
            policy.height,
            policy.workers,
            policy.active_workers,
            &|_| Ok(0),
        )?;
        let bands = bands_for_output_height(dimensions, policy.height).expect("validated rows");
        let active = policy
            .workers
            .active_workers(policy.active_workers, bands.len() as u32);
        let largest =
            ImageDimensions::new(dimensions.width(), policy.height.min(dimensions.height()))
                .expect("nonempty rows");
        // Each joined worker may own its carrier, colour memo and prepared chain.
        // Global recipe resolution remains charged separately by the caller.
        let scratch = effects::carrier_bytes(effects, largest)
            .max(effects::carrier_after_bytes(effects, largest));
        Ok(Some(Self {
            dimensions,
            policy,
            metadata,
            capacity: metadata + u64::from(active) * scratch,
        }))
    }

    pub(super) fn capacity_bytes(&self) -> u64 {
        self.capacity
    }

    pub(super) fn apply(
        &self,
        output: &mut [u8],
        steps: &[ResolvedStep<'_>],
        context: &EffectContext<'_>,
        mask: Option<(&[u8], &[effects::mask::MaskCurve])>,
    ) -> Result<(), Failure> {
        let mut work = RowBandBuffers::<()>::try_new(
            self.dimensions,
            self.policy.height,
            self.policy.workers,
            self.policy.active_workers,
            self.metadata,
            &|_| Ok(0),
        )?;
        let width = self.dimensions.width();
        let palette = context.palette;
        let space = context.space;
        work.execute(
            output,
            width as usize * 4,
            &|band, output, _| {
                // Resolved recipes are immutable and pointwise. The global analysis
                // cache remains caller-owned and never crosses a worker boundary.
                let context = EffectContext {
                    palette,
                    space,
                    analyses: None,
                };
                let dimensions =
                    ImageDimensions::new(width, band.height()).expect("validated band");
                match mask {
                    Some((source, mask)) => {
                        let first = band.y_start() as usize * width as usize * 4;
                        effects::write_mask(
                            &source[first..first + output.len()],
                            dimensions,
                            steps,
                            mask,
                            &context,
                            output,
                        )
                    }
                    None => effects::apply_in_place(output, dimensions, steps, &context),
                }
                .map_err(|_| super::unavailable())?;
                Ok(u64::from(band.height()))
            },
            &mut |_| Ok(()),
        )
    }
}
