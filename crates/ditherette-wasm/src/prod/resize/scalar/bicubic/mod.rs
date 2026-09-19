//! Packed RGBA8 production bicubic resize.
//!
//! Bicubic is represented as a Catmull-Rom cubic reconstruction kernel applied
//! through the production convolution engine.

mod filter;

use crate::image::{ImageDimensions, ImageView, ImageViewMut, Rgba8};
use crate::prod::{contract::failure::Failure, resize::common::allocation::CapacityBudget};

use super::convolution::{
    resize_convolution_rgba8_into, resize_convolution_rgba8_rows_into,
    resize_convolution_rgba8_rows_with_plan_into, resize_convolution_rgba8_with_plan_into,
    ConvolutionResizePlan, ResizeAnchor, SupportPolicy,
};

/// Reusable Catmull-Rom bicubic resize metadata for one source/output shape.
pub struct BicubicResizePlan {
    inner: ConvolutionResizePlan,
}

impl BicubicResizePlan {
    /// Conservative nested-plan and full-call scratch capacity, excluding inline headers.
    pub fn required_bytes(
        source: ImageDimensions,
        output: ImageDimensions,
        policy: SupportPolicy,
    ) -> Result<u64, Failure> {
        ConvolutionResizePlan::required_bytes(source, output, &filter::CATMULL_ROM, policy)
    }

    /// Prepare the landed Catmull-Rom taps through the caller's allocation ledger.
    pub fn try_new(
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: ResizeAnchor,
        policy: SupportPolicy,
        budget: &mut CapacityBudget,
    ) -> Result<Self, Failure> {
        Ok(Self {
            inner: ConvolutionResizePlan::try_new(
                source,
                output,
                anchor,
                &filter::CATMULL_ROM,
                policy,
                budget,
            )?,
        })
    }

    /// Actual heap capacity owned by the plan, excluding caller-owned scratch.
    pub fn capacity_bytes(&self) -> u64 {
        self.inner.capacity_bytes()
    }

    /// Required f64 scratch elements for the landed full-call dispatch.
    pub fn scratch_elements(&self) -> Result<usize, Failure> {
        self.inner.scratch_elements()
    }

    /// Builds reusable coordinate metadata for packed RGBA8 bicubic resize.
    pub fn new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
        support_policy: SupportPolicy,
    ) -> Self {
        Self {
            inner: ConvolutionResizePlan::new(
                source_dimensions,
                output_dimensions,
                anchor,
                &filter::CATMULL_ROM,
                support_policy,
            ),
        }
    }
}

/// Execute landed bicubic with preallocated plan and scratch; failures leave output unchanged.
pub fn resize_bicubic_rgba8_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BicubicResizePlan,
    scratch: &mut [f64],
) -> Result<(), Failure> {
    super::convolution::resize_convolution_rgba8_with_plan_and_scratch_into(
        source,
        output,
        &plan.inner,
        scratch,
    )
}

/// Resizes packed RGBA8 `source` into packed RGBA8 `output` with Catmull-Rom bicubic filtering.
pub fn resize_bicubic_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    resize_convolution_rgba8_into(source, output, anchor, filter::CATMULL_ROM, support_policy);
}

/// Resize one full-width output row range with Catmull-Rom bicubic filtering.
///
/// `full_output_dimensions` is the complete resize target, while `output`
/// stores the local row band starting at absolute output row `y_start`.
pub fn resize_bicubic_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    y_start: u32,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    resize_convolution_rgba8_rows_into(
        source,
        output,
        full_output_dimensions,
        y_start,
        anchor,
        filter::CATMULL_ROM,
        support_policy,
    );
}

/// Resize one full-width output row range with a cached bicubic plan.
pub fn resize_bicubic_rgba8_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BicubicResizePlan,
    y_start: u32,
) {
    resize_convolution_rgba8_rows_with_plan_into(source, output, &plan.inner, y_start);
}

/// Resizes packed RGBA8 `source` into packed RGBA8 `output` with a cached bicubic plan.
pub fn resize_bicubic_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BicubicResizePlan,
) {
    resize_convolution_rgba8_with_plan_into(source, output, &plan.inner);
}
