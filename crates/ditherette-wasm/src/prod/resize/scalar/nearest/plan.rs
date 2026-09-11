//! Nearest-neighbor resize planning.
//!
//! Planning is separated from the packed RGBA8 kernel so the public nearest API
//! can stay small: build or reuse a plan, validate the shared production resize
//! boundary, then execute a nearest kernel. The plan stores only shape-derived
//! metadata; it does not inspect pixels or own row-layout validation.

use crate::image::{rgba8, ImageDimensions};
use std::mem::size_of;

use super::{
    alignment::{axis_coordinate_map, map_axis_coordinate, ResizeAnchor},
    scale::{
        alignment_offset, build_source_x_copy_spans_into, exact_downscale_factors,
        exact_upscale_factors, nearest_scale_class, source_x_copy_spans, uses_source_x_copy_spans,
        NearestScaleClass, SourceXCopySpan,
    },
};

// ACCEPT(perf): Identity pass-through is now represented in the default
// `nearest` profile and one-shot public subject; copying directly improved
// identity cases by roughly 75-180% with other cases neutral/noisy in
// `ditherette-bench run nearest`.
// REJECT(perf): A generic exact-upscale span-fill path regressed 2x by -21.03%
// in `ditherette-bench run nearest`; wider upscale gains do
// not justify hurting the common 2x case.
// ACCEPT(perf): `nearest-anisotropic` now covers width-only and height-only
// nearest profiles with exact correctness and manifest fixtures before testing
// scalar path splits separately from `ditherette-bench run nearest`.
// ACCEPT(perf): Same-width nearest now copies whole source rows for height-only
// anisotropic resizes; `nearest-anisotropic` improved same-width cases by roughly
// 20-180%.
// REJECT(perf): Same-height nearest with a dedicated x-only loop was neutral for
// x-only anisotropic cases and did not justify an extra dispatch path.
// REJECT(perf): Adding a same-width RGBA8 row-copy branch before the hot path
// found no represented 1x/same-width case in the current nearest profile and
// regressed most measured cases in `ditherette-bench run nearest`; do not retry
// without a dedicated identity/same-width subject.
// REJECT(perf): Precomputing packed nearest y/output row byte offsets passed
// correctness but was neutral/noisy and regressed large near-identity downscale
// by -2.06% in `ditherette-bench run nearest`; keep row byte math local to the
// loops until repeated-y run metadata is actually used.
// REJECT(perf): Routing exact 0.75x downscales through span-copy lowered the
// average-span threshold only for that shape but regressed represented large
// downscales by -66.48% in `ditherette-bench run nearest`; keep the stricter
// near-identity span-copy gate.
// NOTE(perf): A separable nearest two-pass is not a current TODO. X/Y mapping is
// already planned independently, and materializing an intermediate image would
// add a full write/read without reducing sampling work. Revisit only if a future
// `nearest` profile exposes a shape where axis-only materialization avoids more
// work than it adds.

/// Reusable nearest-neighbor resize metadata for one source/output shape.
///
/// The plan caches x/y coordinate maps and scale-class decisions that are
/// independent of the input pixels. Benchmarks reuse plans for repeated cases;
/// callers must use a plan whose source dimensions, output dimensions, and
/// anchor match the current resize request.
pub struct NearestResizePlan {
    pub(super) source_dimensions: ImageDimensions,
    pub(super) output_dimensions: ImageDimensions,
    pub(super) anchor: ResizeAnchor,
    pub(super) x_source_starts: Vec<usize>,
    pub(super) y_coordinates: Vec<u32>,
    pub(super) exact_downscale: Option<(u32, u32)>,
    pub(super) exact_upscale: Option<(u32, u32)>,
    pub(super) source_x_copy_spans: Vec<SourceXCopySpan>,
    pub(super) scale_class: NearestScaleClass,
}

/// Allocation failures at the public ownership boundary, without allocating error text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanAllocationError {
    MemoryLimit,
    Allocation,
}

impl NearestResizePlan {
    /// Maximum metadata capacity reserved by the fallible constructor, excluding this record.
    /// Exact integer factor paths need no maps. No temporary coordinate Vec is allocated.
    pub fn required_capacity_bytes(source: ImageDimensions, output: ImageDimensions) -> u64 {
        if exact_downscale_factors(
            source.width(),
            source.height(),
            output.width(),
            output.height(),
        )
        .is_some()
            || exact_upscale_factors(
                source.width(),
                source.height(),
                output.width(),
                output.height(),
            )
            .is_some()
        {
            return 0;
        }
        let maps = u64::from(output.width()) * size_of::<usize>() as u64
            + u64::from(output.height()) * size_of::<u32>() as u64;
        let spans = if uses_source_x_copy_spans(
            source.width(),
            source.height(),
            output.width(),
            output.height(),
        ) {
            u64::from(output.width()) * size_of::<SourceXCopySpan>() as u64
        } else {
            0
        };
        maps + spans
    }

    /// Build the landed metadata with fallible, budgeted reservations.
    /// Pixel mapping, span construction, classification, and execution reuse the landed helpers.
    pub fn try_new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
        memory_limit: u64,
    ) -> Result<Self, PlanAllocationError> {
        let required = Self::required_capacity_bytes(source_dimensions, output_dimensions);
        if required > memory_limit {
            return Err(PlanAllocationError::MemoryLimit);
        }
        let (sw, sh) = (source_dimensions.width(), source_dimensions.height());
        let (ow, oh) = (output_dimensions.width(), output_dimensions.height());
        let exact_downscale = exact_downscale_factors(sw, sh, ow, oh);
        let exact_upscale = exact_upscale_factors(sw, sh, ow, oh);
        let skip = exact_downscale.is_some() || exact_upscale.is_some();
        let mut x_source_starts = Vec::new();
        let mut y_coordinates = Vec::new();
        let mut source_x_copy_spans = Vec::new();
        let mut reserved = 0;
        let mut remaining = required;
        if !skip {
            reserve_metadata(
                &mut x_source_starts,
                ow as usize,
                &mut reserved,
                &mut remaining,
                memory_limit,
            )?;
            reserve_metadata(
                &mut y_coordinates,
                oh as usize,
                &mut reserved,
                &mut remaining,
                memory_limit,
            )?;
            let spans = uses_source_x_copy_spans(sw, sh, ow, oh);
            if spans {
                reserve_metadata(
                    &mut source_x_copy_spans,
                    ow as usize,
                    &mut reserved,
                    &mut remaining,
                    memory_limit,
                )?;
            }
            let (x_alignment, y_alignment) = anchor.axes();
            x_source_starts.extend((0..ow).map(|x| {
                map_axis_coordinate(x, sw, ow, x_alignment) as usize * rgba8::RGBA8_CHANNELS
            }));
            y_coordinates.extend((0..oh).map(|y| map_axis_coordinate(y, sh, oh, y_alignment)));
            if spans {
                build_source_x_copy_spans_into(&x_source_starts, &mut source_x_copy_spans);
            }
        }
        let scale_class = nearest_scale_class(
            sw,
            sh,
            ow,
            oh,
            exact_downscale,
            exact_upscale,
            &source_x_copy_spans,
        );
        Ok(Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_source_starts,
            y_coordinates,
            exact_downscale,
            exact_upscale,
            source_x_copy_spans,
            scale_class,
        })
    }

    /// Actual owned metadata capacity. The public processor adds its other live allocations.
    pub fn capacity_bytes(&self) -> u64 {
        (self.x_source_starts.capacity() * size_of::<usize>()
            + self.y_coordinates.capacity() * size_of::<u32>()
            + self.source_x_copy_spans.capacity() * size_of::<SourceXCopySpan>()) as u64
    }

    /// Write packed source byte offsets as little-endian u32 values for a borrowed gather.
    /// The caller supplies four bytes per output pixel and validates the source byte length.
    pub(crate) fn write_source_offsets(&self, offsets: &mut [u8]) {
        let width = self.output_dimensions.width() as usize;
        let source_stride = self.source_dimensions.width() * 4;
        let (x_anchor, y_anchor) = self.anchor.axes();
        for (y, row) in offsets.chunks_exact_mut(width * 4).enumerate() {
            let source_y = if let Some((_, factor)) = self.exact_downscale {
                y as u32 * factor + alignment_offset(factor, y_anchor)
            } else if let Some((_, factor)) = self.exact_upscale {
                y as u32 / factor
            } else {
                self.y_coordinates[y]
            };
            for (x, offset) in row.chunks_exact_mut(4).enumerate() {
                let source_x = if let Some((factor, _)) = self.exact_downscale {
                    (x as u32 * factor + alignment_offset(factor, x_anchor)) * 4
                } else if let Some((factor, _)) = self.exact_upscale {
                    (x as u32 / factor) * 4
                } else {
                    self.x_source_starts[x] as u32
                };
                offset.copy_from_slice(&(source_y * source_stride + source_x).to_le_bytes());
            }
        }
    }

    /// Build reusable coordinate metadata for one nearest-neighbor resize shape.
    ///
    /// The plan assumes production RGBA8 layout when it computes byte offsets
    /// for x coordinates. Packed-row validation happens at the resize boundary,
    /// not during plan construction.
    pub fn new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> Self {
        let (x_alignment, y_alignment) = anchor.axes();
        let exact_downscale = exact_downscale_factors(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
        );
        let exact_upscale = exact_upscale_factors(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
        );
        let has_factor_fast_path = exact_downscale.is_some() || exact_upscale.is_some();
        let x_source_starts = coordinate_byte_map(
            source_dimensions.width(),
            output_dimensions.width(),
            x_alignment,
            has_factor_fast_path,
        );
        let y_coordinates = coordinate_map(
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
            has_factor_fast_path,
        );
        let source_x_copy_spans = source_x_copy_spans(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
            &x_source_starts,
        );
        let scale_class = nearest_scale_class(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
            exact_downscale,
            exact_upscale,
            &source_x_copy_spans,
        );

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_source_starts,
            y_coordinates,
            exact_downscale,
            exact_upscale,
            source_x_copy_spans,
            scale_class,
        }
    }

    /// Return whether this plan was built for the given shape and anchor.
    pub fn matches(
        &self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> bool {
        self.source_dimensions == source_dimensions
            && self.output_dimensions == output_dimensions
            && self.anchor == anchor
    }

    pub(super) fn same_width(&self) -> bool {
        self.source_dimensions.width() == self.output_dimensions.width()
    }
}

fn reserve_metadata<T>(
    buffer: &mut Vec<T>,
    count: usize,
    reserved: &mut u64,
    remaining: &mut u64,
    limit: u64,
) -> Result<(), PlanAllocationError> {
    buffer
        .try_reserve_exact(count)
        .map_err(|_| PlanAllocationError::Allocation)?;
    *remaining -= count as u64 * size_of::<T>() as u64;
    *reserved += buffer.capacity() as u64 * size_of::<T>() as u64;
    if *reserved + *remaining > limit {
        return Err(PlanAllocationError::MemoryLimit);
    }
    Ok(())
}

fn coordinate_byte_map(
    source_len: u32,
    output_len: u32,
    alignment: super::alignment::AxisAlignment,
    skip: bool,
) -> Vec<usize> {
    if skip {
        return Vec::new();
    }

    axis_coordinate_map(source_len, output_len, alignment)
        .into_iter()
        .map(|source_x| source_x as usize * rgba8::RGBA8_CHANNELS)
        .collect()
}

fn coordinate_map(
    source_len: u32,
    output_len: u32,
    alignment: super::alignment::AxisAlignment,
    skip: bool,
) -> Vec<u32> {
    if skip {
        Vec::new()
    } else {
        axis_coordinate_map(source_len, output_len, alignment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::{ImageView, ImageViewMut, Rgba8};

    #[test]
    fn fallible_metadata_and_outputs_equal_landed_plans_for_all_classes_and_anchors() {
        let anchors = [
            ResizeAnchor::TopLeft,
            ResizeAnchor::Top,
            ResizeAnchor::TopRight,
            ResizeAnchor::Left,
            ResizeAnchor::Center,
            ResizeAnchor::Right,
            ResizeAnchor::BottomLeft,
            ResizeAnchor::Bottom,
            ResizeAnchor::BottomRight,
        ];
        for (sw, sh, ow, oh) in [
            (8, 6, 4, 3),
            (2, 3, 6, 9),
            (21, 21, 20, 20),
            (7, 5, 3, 2),
            (3, 2, 7, 5),
            (3, 7, 3, 5),
            (4, 3, 4, 3),
        ] {
            let source = ImageDimensions::new(sw, sh).unwrap();
            let output = ImageDimensions::new(ow, oh).unwrap();
            let input: Vec<u8> = (0..sw * sh * 4).map(|x| (x * 73) as u8).collect();
            for anchor in anchors {
                let original = NearestResizePlan::new(source, output, anchor);
                let budget = NearestResizePlan::required_capacity_bytes(source, output);
                let fallible = NearestResizePlan::try_new(source, output, anchor, budget).unwrap();
                assert_eq!(fallible.capacity_bytes(), budget);
                assert_eq!(fallible.x_source_starts, original.x_source_starts);
                assert_eq!(fallible.y_coordinates, original.y_coordinates);
                assert_eq!(fallible.exact_downscale, original.exact_downscale);
                assert_eq!(fallible.exact_upscale, original.exact_upscale);
                assert_eq!(fallible.scale_class, original.scale_class);
                let spans = |plan: &NearestResizePlan| {
                    plan.source_x_copy_spans
                        .iter()
                        .map(|span| (span.source_start, span.output_start, span.byte_len))
                        .collect::<Vec<_>>()
                };
                assert_eq!(spans(&fallible), spans(&original));
                let mut expected = vec![0; (ow * oh * 4) as usize];
                let mut actual = expected.clone();
                for (plan, bytes) in [(&original, &mut expected), (&fallible, &mut actual)] {
                    super::super::resize_nearest_rgba8_with_plan_into(
                        ImageView::<Rgba8>::packed(&input, source).unwrap(),
                        ImageViewMut::packed(bytes, output).unwrap(),
                        plan,
                    );
                }
                assert_eq!(actual, expected);
                if budget > 0 {
                    assert!(matches!(
                        NearestResizePlan::try_new(source, output, anchor, budget - 1),
                        Err(PlanAllocationError::MemoryLimit)
                    ));
                }
            }
        }
    }
}
