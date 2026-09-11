//! Shared materialized resize/perturb/indexed stages around the existing kernels.

mod cache;

use super::{
    identity, perturb,
    preparation::{Call, ResizePreparation, Store},
    processor::Allocator,
    quantize::{QuantizeBoundary, QuantizeRequest},
};
use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            cache::StageOptions,
            failure::Failure,
            lifecycle::Stage,
            request::{BayerSize, DiffusionFeedback, DitherPolicy, Output, Placement},
        },
        dither::error_diffusion::prepared::{execute_with_placement_progress, DiffusionPolicy},
        dither::placement::AdaptivePlacementWork,
        dither::yiluoma::row_bands::YliluomaBands,
    },
};

/// Settings and storage have already passed their method-specific validation.
pub(super) fn run<B: QuantizeBoundary, A: Allocator>(
    request: QuantizeRequest<'_>,
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    resize: Option<Output>,
    dither: DitherPolicy,
    boundary: &mut B,
    allocator: &mut A,
    limit: u64,
    overhead: u64,
    peak: &mut u64,
    store: &mut Store,
) -> Result<B::Output, Failure> {
    // Process has no measured S36 complete-call class; explicit developer bands still apply.
    let measured = if resize.is_none() {
        super::row_fields::measured_indexed(
            output_dimensions,
            request,
            dither,
            super::execution::worker_budget(),
        )
    } else {
        None
    };
    let row_policy = store.row_policy(super::execution::ExecutionStage::Indexed, measured);
    let mixing_policy = store.row_policy(
        super::execution::ExecutionStage::Mixing,
        crate::prod::dither::yiluoma::policy::measured(
            output_dimensions,
            request,
            dither,
            resize,
            super::execution::worker_budget(),
        ),
    );
    // All supported resize policies already define equal dimensions as exact identity.
    // Keep that identity in the dependency chain without materializing another image.
    let resize = resize.filter(|_| source_dimensions != output_dimensions);
    let enabled = boundary.progress().is_some();
    let mut progress = super::progress::Control::new(enabled);
    progress.report(boundary.progress(), Stage::Prepare, 0, 1)?;
    let source_len = source_dimensions
        .storage_len::<Rgba8>()
        .expect("validated source");
    let rgba_len = output_dimensions
        .storage_len::<Rgba8>()
        .expect("validated output");
    let sparse = boundary.supports_sparse_input()
        && resize.is_some_and(|output| {
            super::resize::sparse_nearest(source_len, rgba_len, output.resize)
        });
    let offset_bytes = super::resize::sparse_nearest_offset_bytes(output_dimensions);
    let (mut call, source) = if sparse {
        // Clear the snapshot length before reusing its storage for source offsets.
        // Sparse samples have no full-source identity and never enter the image cache.
        let mut call = Call::new(
            store, None, None, [0; 4], 0, overhead, limit, peak, allocator,
        )?;
        // Offset scratch must not pin a larger prior snapshot during the remaining preflight.
        if call.scratch.buffers[0].capacity() > offset_bytes {
            call.scratch.buffers[0] = Vec::new();
        }
        (call, None)
    } else {
        let mut call = Call::snapshot(store, source_len, overhead, limit, peak, allocator)?;
        let source = call.source(source_dimensions, |bytes, compare| {
            boundary.snapshot_input(bytes, compare)
        })?;
        (call, Some(source))
    };
    let resize_key = resize
        .zip(source)
        .map(|(output, source)| identity::stage(Some(source), StageOptions::Resize { output }))
        .transpose()?;
    let rgba_key = resize_key.or(source);
    let policy = match dither {
        DitherPolicy::Separable { perturb } => Some(perturb),
        _ => None,
    };
    let perturb_key = policy
        .zip(rgba_key)
        .map(|(perturb, rgba_key)| {
            identity::stage(Some(rgba_key), StageOptions::Perturb { perturb })
        })
        .transpose()?;
    let indexed_dither = if policy.is_some() {
        DitherPolicy::None {}
    } else {
        dither
    };
    let final_key = perturb_key
        .or(rgba_key)
        .map(|parent| {
            identity::indexed(
                parent,
                identity::palette_content(request.palette)?,
                request.alpha,
                request.matching,
                indexed_dither,
            )
        })
        .transpose()?;
    if final_key.is_some_and(|key| call.take_image(2, key)) {
        let (bytes, metadata) = call.indexed_result(3);
        let result = boundary.complete(bytes, output_dimensions, metadata);
        return call.finish(progress.finish(result, boundary.progress()));
    }
    // A downstream hit needs no upstream materialization, even after upstream eviction.
    let perturbed_hit = perturb_key.is_some_and(|key| call.take_image(1, key));
    let resize = resize.filter(|_| !perturbed_hit);
    let resize_key = resize_key.filter(|_| !perturbed_hit);
    let resized_hit = resize_key.is_some_and(|key| call.take_image(0, key));
    let mixing_policy = mixing_policy.filter(|_| matches!(dither, DitherPolicy::Yliluoma { .. }));
    let mixing_capacity = mixing_policy
        .map(|policy| {
            YliluomaBands::required_bytes(
                output_dimensions,
                policy.height,
                policy.workers,
                policy.active_workers,
            )
        })
        .transpose()?
        .unwrap_or(0);
    call.charge_working_capacity(mixing_capacity, peak)?;
    let band_plan = row_policy
        .filter(|_| {
            matches!(
                dither,
                DitherPolicy::None {} | DitherPolicy::Separable { .. }
            )
        })
        .map(|row_policy| {
            super::row_fields::Plan::new(
                output_dimensions,
                row_policy,
                policy.is_some() && !perturbed_hit,
            )
        })
        .transpose()?;
    let band_capacity = band_plan
        .as_ref()
        .map_or(0, |plan| plan.additional_capacity());
    call.charge_working_capacity(band_capacity, peak)?;
    call.prepare(
        Some(request),
        resize
            .filter(|_| !resized_hit)
            .map(|output| ResizePreparation {
                source: source_dimensions,
                output,
            }),
        [
            if sparse { offset_bytes } else { source_len },
            if resize.is_some() && !resized_hit {
                rgba_len
            } else {
                0
            },
            if policy.is_some() && !perturbed_hit {
                rgba_len
            } else {
                0
            },
            rgba_len / 4,
        ],
        if matches!(dither, DitherPolicy::Diffusion { .. }) {
            output_dimensions.width() as usize * 3
        } else {
            0
        },
        peak,
        allocator,
    )?;
    let mut mixing = mixing_policy
        .map(|policy| {
            YliluomaBands::try_new(
                output_dimensions,
                policy.height,
                policy.workers,
                policy.active_workers,
                mixing_capacity,
            )
        })
        .transpose()?;
    let mut bands = band_plan.as_ref().map(|plan| plan.allocate()).transpose()?;
    let placement_policy = match dither {
        DitherPolicy::Diffusion { placement, .. } => placement,
        DitherPolicy::Separable { perturb } if !perturbed_hit && bands.is_none() => {
            perturb.placement
        }
        _ => Placement::Everywhere {},
    };
    let mut placement = AdaptivePlacementWork::try_new(
        output_dimensions.width(),
        placement_policy,
        call.available_working_capacity(),
    );
    let placement_capacity = placement
        .as_ref()
        .map_or(0, AdaptivePlacementWork::capacity_bytes);
    call.charge_optional_capacity(placement_capacity, peak)?;
    if resize.is_some() && !resized_hit {
        let (_, prepared, scratch) = call.parts();
        let [source, resized, _, _] = &mut scratch.buffers;
        let prepared = prepared.expect("requested resize");
        if sparse {
            let (columns, rows) = prepared.write_nearest_source_offsets(source);
            boundary.gather_input(resized, columns, rows, source_len)?;
            progress.report(
                boundary.progress(),
                Stage::Resize,
                u64::from(output_dimensions.height()),
                u64::from(output_dimensions.height()),
            )?;
        } else {
            let source = ImageView::packed(source, source_dimensions).expect("owned source");
            let output = ImageViewMut::packed(resized, output_dimensions).expect("reserved resize");
            if enabled {
                prepared.execute_with_progress(source, output, &mut |completed, total| {
                    progress.report(
                        boundary.progress(),
                        Stage::Resize,
                        u64::from(completed),
                        u64::from(total),
                    )
                })?;
            } else {
                prepared.execute(source, output)?;
            }
        }
    }
    if let Some(perturb) = policy {
        if !perturbed_hit {
            let (_, _, images, scratch) = call.image_parts();
            let [source, resized, perturbed, _] = &mut scratch.buffers;
            let rgba = images[0].map_or_else(
                || {
                    if resize.is_some() {
                        resized.as_slice()
                    } else {
                        source.as_slice()
                    }
                },
                |image| image.bytes.as_slice(),
            );
            let source = ImageView::packed(rgba, output_dimensions).expect("complete RGBA");
            if bands.is_some() || enabled {
                progress.report(
                    boundary.progress(),
                    Stage::Perturb,
                    0,
                    u64::from(output_dimensions.height()),
                )?;
                let mut report = |completed| {
                    progress.report(
                        boundary.progress(),
                        Stage::Perturb,
                        completed,
                        u64::from(output_dimensions.height()),
                    )
                };
                if let Some(work) = &mut bands {
                    perturb::execute_bands(source, perturbed, perturb, work, &mut report)?;
                } else {
                    let output = ImageViewMut::packed(perturbed, output_dimensions)
                        .expect("reserved perturb");
                    perturb::execute_with_scratch(
                        source,
                        output,
                        perturb,
                        placement
                            .as_mut()
                            .map_or(&mut [], AdaptivePlacementWork::scratch),
                        |completed| report(u64::from(completed)),
                    )?;
                }
            } else {
                let output =
                    ImageViewMut::packed(perturbed, output_dimensions).expect("reserved perturb");
                perturb::execute_with_scratch(
                    source,
                    output,
                    perturb,
                    placement
                        .as_mut()
                        .map_or(&mut [], AdaptivePlacementWork::scratch),
                    |_| Ok(()),
                )?;
            }
        }
    }
    let mut rgb_cache = if matches!(
        dither,
        DitherPolicy::None {}
            | DitherPolicy::Separable { .. }
            | DitherPolicy::Diffusion {
                feedback: DiffusionFeedback::SrgbBytes,
                ..
            }
    ) {
        cache::Work::try_new(
            output_dimensions,
            row_policy.filter(|_| bands.is_some()),
            call.available_working_capacity(),
        )
    } else {
        None
    };
    let rgb_cache_capacity = rgb_cache.as_ref().map_or(0, cache::Work::capacity_bytes);
    call.charge_optional_capacity(rgb_cache_capacity, peak)?;
    let (prepared, _, images, scratch) = call.image_parts();
    let prepared = prepared.expect("requested palette");
    let [source, resized, perturbed, indices] = &mut scratch.buffers;
    let rgba = if policy.is_some() {
        images[1].map_or(perturbed.as_slice(), |image| &image.bytes)
    } else if resize.is_some() {
        images[0].map_or(resized.as_slice(), |image| &image.bytes)
    } else {
        source.as_slice()
    };
    let view = ImageView::packed(rgba, output_dimensions).expect("complete RGBA");
    let stage = if matches!(
        dither,
        DitherPolicy::Diffusion { .. } | DitherPolicy::Yliluoma { .. }
    ) {
        Stage::DitherAndQuantize
    } else {
        Stage::Quantize
    };
    progress.report(
        boundary.progress(),
        stage,
        0,
        u64::from(output_dimensions.height()),
    )?;
    let mut report_row = |completed| {
        progress.report(
            boundary.progress(),
            stage,
            u64::from(completed),
            u64::from(output_dimensions.height()),
        )
    };
    match dither {
        DitherPolicy::Diffusion { .. } if enabled => execute_with_placement_progress(
            prepared,
            &mut scratch.diffusion,
            match &mut rgb_cache {
                Some(cache::Work::Scalar(entries)) => entries,
                _ => &mut [],
            },
            placement
                .as_mut()
                .map_or(&mut [], AdaptivePlacementWork::scratch),
            view,
            indices,
            DiffusionPolicy::new(dither)?,
            &mut report_row,
        )?,
        DitherPolicy::Diffusion { .. } => execute_with_placement_progress(
            prepared,
            &mut scratch.diffusion,
            match &mut rgb_cache {
                Some(cache::Work::Scalar(entries)) => entries,
                _ => &mut [],
            },
            placement
                .as_mut()
                .map_or(&mut [], AdaptivePlacementWork::scratch),
            view,
            indices,
            DiffusionPolicy::new(dither)?,
            |_| Ok(()),
        )?,
        DitherPolicy::Yliluoma { size, placement } => {
            use crate::prod::dither::ordered::BayerSize as Matrix;
            let matrix = match size {
                BayerSize::Two => Matrix::Two,
                BayerSize::Four => Matrix::Four,
                BayerSize::Eight => Matrix::Eight,
                BayerSize::Sixteen => Matrix::Sixteen,
            };
            if let Some(work) = &mut mixing {
                work.execute(view, prepared, indices, matrix, placement, &mut report_row)?;
            } else if enabled {
                crate::prod::dither::yiluoma::dither_yiluoma_with_progress(
                    view,
                    prepared,
                    indices,
                    matrix,
                    placement,
                    &mut report_row,
                )?;
            } else {
                crate::prod::dither::yiluoma::dither_yiluoma_into(
                    view, prepared, indices, matrix, placement,
                );
            }
        }
        _ if rgb_cache.is_some() => {
            rgb_cache
                .as_mut()
                .unwrap()
                .execute(prepared, view, indices, &mut report_row)?
        }
        _ if bands.is_some() => prepared.quantize_bands_into(
            view,
            indices,
            bands.as_mut().unwrap(),
            &mut |completed| report_row(completed as u32),
        )?,
        _ if enabled => prepared.quantize_with_progress(view, indices, &mut report_row)?,
        _ => prepared.quantize_into(view, indices),
    }
    drop(rgb_cache);
    call.release_working_capacity(rgb_cache_capacity);
    drop(placement);
    call.release_working_capacity(placement_capacity);
    drop(bands);
    call.release_working_capacity(band_capacity);
    drop(mixing);
    call.release_working_capacity(mixing_capacity);
    if let Some(key) = resize_key {
        call.retain_rgba(0, key, 1, output_dimensions, peak);
    }
    if let Some(key) = perturb_key {
        call.retain_rgba(1, key, 2, output_dimensions, peak);
    }
    if let Some(key) = final_key {
        call.retain_indexed(key, 3, output_dimensions, peak);
    }
    let (bytes, metadata) = call.indexed_result(3);
    let result = boundary.complete(bytes, output_dimensions, metadata);
    call.finish(progress.finish(result, boundary.progress()))
}
