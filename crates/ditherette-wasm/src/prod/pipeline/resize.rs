//! Bounded full-call preparation and dispatch for landed resize kernels.

use std::{mem::size_of, num::NonZeroU32};

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
            scalar::{
                area, bicubic, bilinear, convolution, lanczos, nearest,
                trilinear::PreparedTrilinear,
            },
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
    Trilinear {
        scratch: Option<PreparedTrilinear<Rgba8>>,
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: Anchor,
    },
}

// PreparedResize's inline storage belongs to the preparation entry's reserved Vec.
const TRILINEAR_RECORD_BYTES: u64 = size_of::<PreparedTrilinear<Rgba8>>() as u64;

pub(super) fn supported(policy: ResizePolicy) -> Result<(), Failure> {
    match policy {
        ResizePolicy::Nearest { .. }
        | ResizePolicy::Area {}
        | ResizePolicy::Bilinear { .. }
        | ResizePolicy::Bicubic { .. }
        | ResizePolicy::Lanczos2 { .. }
        | ResizePolicy::Lanczos3 { .. }
        | ResizePolicy::Trilinear { .. } => Ok(()),
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
            ResizePolicy::Trilinear { .. } => {
                PreparedTrilinear::<Rgba8>::required_bytes(source, output)
                    .map(|bytes| bytes - TRILINEAR_RECORD_BYTES)
            }
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
            ResizePolicy::Trilinear { anchor } => PreparedTrilinear::try_new(
                source,
                output,
                bilinear_anchor(anchor),
                limit + TRILINEAR_RECORD_BYTES,
            )
            .map(|scratch| Self::Trilinear {
                scratch: Some(scratch),
                source,
                output,
                anchor,
            }),
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
            Self::Trilinear { scratch, .. } => scratch
                .as_ref()
                .map_or(0, |plan| plan.capacity_bytes() - TRILINEAR_RECORD_BYTES),
        }
    }

    /// Mutable storage is outside the retained preparation cap and is evicted first.
    pub(super) fn scratch_capacity_bytes(&self) -> u64 {
        match self {
            Self::Area(_, scratch) | Self::Bilinear(_, scratch) => scratch.capacity() as u64 * 4,
            Self::Bicubic(_, scratch) | Self::Lanczos(_, scratch) => scratch.capacity() as u64 * 8,
            Self::Trilinear { .. } => self.capacity_bytes(),
            _ => 0,
        }
    }

    pub(super) fn required_scratch_bytes(&self) -> Result<u64, Failure> {
        Ok(match self {
            Self::Area(plan, _) => plan.scratch_elements() as u64 * 4,
            Self::Bilinear(plan, _) => plan.scratch_elements() as u64 * 4,
            Self::Bicubic(plan, _) => plan.scratch_elements()? as u64 * 8,
            Self::Lanczos(plan, _) => plan.scratch_elements()? as u64 * 8,
            Self::Trilinear { source, output, .. } => {
                PreparedTrilinear::<Rgba8>::required_bytes(*source, *output)?
                    - TRILINEAR_RECORD_BYTES
            }
            _ => 0,
        })
    }

    pub(super) fn drop_scratch(&mut self) {
        match self {
            Self::Area(_, scratch) | Self::Bilinear(_, scratch) => *scratch = Vec::new(),
            Self::Bicubic(_, scratch) | Self::Lanczos(_, scratch) => *scratch = Vec::new(),
            Self::Trilinear { scratch, .. } => *scratch = None,
            _ => {}
        }
    }

    /// Rebuild released scratch with the landed fallible helpers, before source import.
    pub(super) fn restore_scratch(&mut self, limit: u64) -> Result<(), Failure> {
        let retained = self.capacity_bytes() - self.scratch_capacity_bytes();
        let mut budget =
            CapacityBudget::new(limit.checked_sub(retained).ok_or_else(|| {
                Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
            })?);
        match self {
            Self::Area(plan, scratch) => {
                restore_vector(scratch, plan.scratch_elements(), &mut budget)?
            }
            Self::Bilinear(plan, scratch) => {
                restore_vector(scratch, plan.scratch_elements(), &mut budget)?
            }
            Self::Bicubic(plan, scratch) => {
                restore_vector(scratch, plan.scratch_elements()?, &mut budget)?
            }
            Self::Lanczos(plan, scratch) => {
                restore_vector(scratch, plan.scratch_elements()?, &mut budget)?
            }
            Self::Trilinear {
                scratch,
                source,
                output,
                anchor,
            } if scratch.is_none() => {
                *scratch = Some(PreparedTrilinear::try_new(
                    *source,
                    *output,
                    bilinear_anchor(*anchor),
                    limit + TRILINEAR_RECORD_BYTES,
                )?);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn execute(
        &mut self,
        source: ImageView<'_, Rgba8>,
        output: ImageViewMut<'_, Rgba8>,
    ) -> Result<(), Failure> {
        self.execute_with_progress(source, output, &mut |_, _| Ok(()))
    }

    /// Reuses each prepared kernel and reports only work that its chosen path performs.
    pub(super) fn execute_with_progress(
        &mut self,
        source: ImageView<'_, Rgba8>,
        mut output: ImageViewMut<'_, Rgba8>,
        progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        let height = output.dimensions().height();
        match self {
            Self::Identity => {
                progress(0, height)?;
                output.data_mut().copy_from_slice(source.data());
                progress(height, height)?;
            }
            Self::Nearest(plan) => {
                // Keep its optimized repeat/copy dispatch intact; this whole call is one work batch.
                progress(0, height)?;
                nearest::resize_nearest_rgba8_with_plan_into(source, output, plan);
                progress(height, height)?;
            }
            Self::Area(plan, scratch) => {
                progress(0, height)?;
                area::resize_area_with_progress(source, output, plan, scratch, &mut |rows| {
                    progress(rows, height)
                })?;
            }
            Self::Bilinear(plan, scratch) => {
                progress(0, height)?;
                bilinear::resize_bilinear_with_progress(
                    source,
                    output,
                    plan,
                    scratch,
                    &mut |rows| progress(rows, height),
                )?;
            }
            Self::Bicubic(plan, scratch) => {
                bicubic::resize_bicubic_with_progress(source, output, plan, scratch, progress)?
            }
            Self::Lanczos(plan, scratch) => {
                lanczos::resize_lanczos_with_progress(source, output, plan, scratch, progress)?
            }
            Self::Trilinear { scratch, .. } => scratch
                .as_mut()
                .expect("reserved trilinear scratch")
                .execute_with_progress(source, output, progress)?,
        }
        Ok(())
    }
}

fn restore_vector<T: Default + Clone>(
    scratch: &mut Vec<T>,
    length: usize,
    budget: &mut CapacityBudget,
) -> Result<(), Failure> {
    if scratch.capacity() < length {
        *scratch = budget.vector(length)?;
    } else {
        budget.check_additional((scratch.capacity() * size_of::<T>()) as u64)?;
    }
    scratch.resize(length, T::default());
    Ok(())
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
