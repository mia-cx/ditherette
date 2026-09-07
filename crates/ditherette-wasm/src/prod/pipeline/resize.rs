//! Bounded full-call preparation and dispatch for landed resize kernels.

use std::num::NonZeroU32;

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{Anchor, ResizePolicy, Support},
        },
        resize::{
            common::allocation::CapacityBudget,
            scalar::{area, bicubic, bilinear, convolution, lanczos, nearest},
        },
    },
};

pub(super) enum PreparedResize {
    Identity,
    Nearest(nearest::NearestResizePlan),
    Area(area::AreaResizePlan, Vec<f32>),
    Bilinear(bilinear::BilinearResizePlan, Vec<f32>),
    Bicubic(bicubic::BicubicResizePlan, Vec<f64>),
    Lanczos(lanczos::LanczosResizePlan, Vec<f64>),
}

pub(super) fn supported(policy: ResizePolicy) -> Result<(), Failure> {
    match policy {
        ResizePolicy::Nearest { .. }
        | ResizePolicy::Area {}
        | ResizePolicy::Bilinear { .. }
        | ResizePolicy::Bicubic { .. }
        | ResizePolicy::Lanczos2 { .. }
        | ResizePolicy::Lanczos3 { .. } => Ok(()),
        ResizePolicy::Trilinear { .. } => Err(Failure::new(
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
            ResizePolicy::Bicubic { support, .. } => bicubic::BicubicResizePlan::required_bytes(
                source,
                output,
                convolution_support(support),
            ),
            ResizePolicy::Lanczos2 { support, .. } | ResizePolicy::Lanczos3 { support, .. } => {
                let radius = if matches!(policy, ResizePolicy::Lanczos2 { .. }) {
                    2
                } else {
                    3
                };
                lanczos::LanczosResizePlan::required_bytes(
                    source,
                    output,
                    NonZeroU32::new(radius).unwrap(),
                    convolution_support(support),
                )
            }
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
            ResizePolicy::Bicubic { anchor, support } => {
                let plan = bicubic::BicubicResizePlan::try_new(
                    source,
                    output,
                    convolution_anchor(anchor),
                    convolution_support(support),
                    &mut budget,
                )?;
                let elements = plan.scratch_elements()?;
                let mut scratch = budget.vector(elements)?;
                scratch.resize(elements, 0.0);
                Ok(Self::Bicubic(plan, scratch))
            }
            ResizePolicy::Lanczos2 { anchor, support }
            | ResizePolicy::Lanczos3 { anchor, support } => {
                let constructor = if matches!(policy, ResizePolicy::Lanczos2 { .. }) {
                    lanczos::LanczosResizePlan::try_new2
                } else {
                    lanczos::LanczosResizePlan::try_new3
                };
                let plan = constructor(
                    source,
                    output,
                    convolution_anchor(anchor),
                    convolution_support(support),
                    &mut budget,
                )?;
                let elements = plan.scratch_elements()?;
                let mut scratch = budget.vector(elements)?;
                scratch.resize(elements, 0.0);
                Ok(Self::Lanczos(plan, scratch))
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
            Self::Bicubic(plan, scratch) => plan.capacity_bytes() + scratch.capacity() as u64 * 8,
            Self::Lanczos(plan, scratch) => plan.capacity_bytes() + scratch.capacity() as u64 * 8,
        }
    }

    pub(super) fn execute(
        &mut self,
        source: ImageView<'_, Rgba8>,
        mut output: ImageViewMut<'_, Rgba8>,
    ) -> Result<(), Failure> {
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
            Self::Bicubic(plan, scratch) => {
                bicubic::resize_bicubic_rgba8_with_plan_and_scratch_into(
                    source, output, plan, scratch,
                )?
            }
            Self::Lanczos(plan, scratch) => {
                lanczos::resize_lanczos_rgba8_with_plan_and_scratch_into(
                    source, output, plan, scratch,
                )?
            }
        }
        Ok(())
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

fn convolution_anchor(anchor: Anchor) -> convolution::ResizeAnchor {
    use convolution::ResizeAnchor as A;
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

fn convolution_support(support: Support) -> convolution::SupportPolicy {
    match support {
        Support::Fixed => convolution::SupportPolicy::Fixed,
        Support::ScaleAware => convolution::SupportPolicy::ScaleAware,
    }
}
