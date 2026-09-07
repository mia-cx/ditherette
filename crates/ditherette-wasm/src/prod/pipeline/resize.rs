//! Bounded full-call preparation and dispatch for landed resize kernels.

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{Anchor, ResizePolicy},
        },
        resize::{
            common::allocation::CapacityBudget,
            scalar::{area, bilinear, nearest},
        },
    },
};

pub(super) enum PreparedResize {
    Identity,
    Nearest(nearest::NearestResizePlan),
    Area(area::AreaResizePlan, Vec<f32>),
    Bilinear(bilinear::BilinearResizePlan, Vec<f32>),
}

pub(super) fn supported(policy: ResizePolicy) -> Result<(), Failure> {
    match policy {
        ResizePolicy::Nearest { .. } | ResizePolicy::Area {} | ResizePolicy::Bilinear { .. } => {
            Ok(())
        }
        _ => Err(Failure::new(
            ErrorCode::UnsupportedOperation,
            ErrorPath::OutputResize,
        )),
    }
}

impl PreparedResize {
    pub(super) fn required_bytes(
        source: ImageDimensions,
        output: ImageDimensions,
        policy: ResizePolicy,
    ) -> Result<u64, Failure> {
        supported(policy)?;
        if source == output {
            return Ok(0);
        }
        match policy {
            ResizePolicy::Nearest { .. } => Ok(
                nearest::NearestResizePlan::required_capacity_bytes(source, output),
            ),
            ResizePolicy::Area {} => area::AreaResizePlan::required_bytes(source, output),
            ResizePolicy::Bilinear { anchor } => bilinear::BilinearResizePlan::required_bytes(
                source,
                output,
                bilinear_anchor(anchor),
            ),
            _ => unreachable!("supported policy checked above"),
        }
    }

    pub(super) fn new(
        source: ImageDimensions,
        output: ImageDimensions,
        policy: ResizePolicy,
        limit: u64,
    ) -> Result<Self, Failure> {
        let mut budget = CapacityBudget::new(limit);
        budget.check_additional(Self::required_bytes(source, output, policy)?)?;
        if source == output {
            return Ok(Self::Identity);
        }
        match policy {
            ResizePolicy::Nearest { anchor } => {
                nearest::NearestResizePlan::try_new(source, output, nearest_anchor(anchor), limit)
                    .map(Self::Nearest)
                    .map_err(|error| match error {
                        nearest::PlanAllocationError::MemoryLimit => {
                            Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
                        }
                        nearest::PlanAllocationError::Allocation => {
                            Failure::new(ErrorCode::WasmMemoryUnavailable, ErrorPath::Wasm)
                        }
                    })
            }
            ResizePolicy::Area {} => {
                let plan = area::AreaResizePlan::try_new(source, output, &mut budget)?;
                let mut scratch = budget.vector(plan.scratch_elements())?;
                scratch.resize(plan.scratch_elements(), 0.0);
                Ok(Self::Area(plan, scratch))
            }
            ResizePolicy::Bilinear { anchor } => {
                let plan = bilinear::BilinearResizePlan::try_new(
                    source,
                    output,
                    bilinear_anchor(anchor),
                    &mut budget,
                )?;
                let mut scratch = budget.vector(plan.scratch_elements())?;
                scratch.resize(plan.scratch_elements(), 0.0);
                Ok(Self::Bilinear(plan, scratch))
            }
            _ => unreachable!("supported policy checked above"),
        }
    }

    pub(super) fn capacity_bytes(&self) -> u64 {
        match self {
            Self::Identity => 0,
            Self::Nearest(plan) => plan.capacity_bytes(),
            Self::Area(plan, scratch) => plan.capacity_bytes() + scratch.capacity() as u64 * 4,
            Self::Bilinear(plan, scratch) => plan.capacity_bytes() + scratch.capacity() as u64 * 4,
        }
    }

    pub(super) fn execute(
        &mut self,
        source: ImageView<'_, Rgba8>,
        mut output: ImageViewMut<'_, Rgba8>,
    ) {
        match self {
            Self::Identity => output.data_mut().copy_from_slice(source.data()),
            Self::Nearest(plan) => {
                nearest::resize_nearest_rgba8_with_plan_into(source, output, plan)
            }
            Self::Area(plan, scratch) => {
                area::resize_area_rgba8_with_plan_and_scratch_into(source, output, plan, scratch)
            }
            Self::Bilinear(plan, scratch) => {
                bilinear::resize_bilinear_rgba8_with_plan_and_scratch_into(
                    source, output, plan, scratch,
                )
            }
        }
    }
}

fn nearest_anchor(anchor: Anchor) -> nearest::alignment::ResizeAnchor {
    use nearest::alignment::ResizeAnchor as A;
    match anchor {
        Anchor::TopLeft => A::TopLeft,
        Anchor::Top => A::Top,
        Anchor::TopRight => A::TopRight,
        Anchor::Left => A::Left,
        Anchor::Center => A::Center,
        Anchor::Right => A::Right,
        Anchor::BottomLeft => A::BottomLeft,
        Anchor::Bottom => A::Bottom,
        Anchor::BottomRight => A::BottomRight,
    }
}

fn bilinear_anchor(anchor: Anchor) -> bilinear::alignment::ResizeAnchor {
    use bilinear::alignment::ResizeAnchor as A;
    match anchor {
        Anchor::TopLeft => A::TopLeft,
        Anchor::Top => A::Top,
        Anchor::TopRight => A::TopRight,
        Anchor::Left => A::Left,
        Anchor::Center => A::Center,
        Anchor::Right => A::Right,
        Anchor::BottomLeft => A::BottomLeft,
        Anchor::Bottom => A::Bottom,
        Anchor::BottomRight => A::BottomRight,
    }
}
