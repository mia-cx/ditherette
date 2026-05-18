use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        shared::convolution::Kernel,
    },
};

// TODO(perf): Store contributions in a flat buffer plus per-output ranges. One
// Vec per output coordinate creates allocation overhead and scattered reads in
// the hot loop.
// TODO(perf): Use fixed-size stack arrays or SmallVec-style storage for compact
// kernels whose tap count is bounded (bicubic/Lanczos2/Lanczos3 upscales) to
// avoid one heap allocation per output coordinate. Benchmark with
// `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before accepting.
// TODO(perf): Split x and y contribution types so x can store byte offsets and y
// can store row offsets without carrying unused generic fields. Benchmark with
// `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before accepting.
#[derive(Debug)]
struct AxisContributions {
    samples: Vec<WeightedSourceIndex>,
}

#[derive(Debug)]
struct WeightedSourceIndex {
    // TODO(perf): Store byte offsets rather than source indices for the x axis,
    // and row byte offsets for the y axis, to avoid repeated offset math.
    index: usize,
    // TODO(perf): Convert normalized weights to fixed-point i16/i32 once output
    // equality expectations are settled; f64 multiplies dominate this reference
    // path for wide filters.
    weight: f64,
}

/// Allocates and resizes with a separable convolution filter.
pub(crate) fn resize_with_convolution<K: Kernel>(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    kernel: K,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
        kernel,
        scale_aware,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-owned output buffer with a separable convolution filter.
// TODO(perf): Implement a true separable two-pass resize. The current generic
// loop combines x*y taps per output pixel; doing horizontal convolution into a
// scratch image and then vertical convolution cuts work to x+y taps.
pub(crate) fn resize_with_convolution_into<K: Kernel>(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    kernel: K,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        output_rgba.copy_from_slice(source_rgba);
        return Ok(());
    }

    // TODO(perf): Add one-axis fast paths for same-width or same-height resizes
    // so pure vertical or horizontal convolution skips building and walking the
    // unchanged axis. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Add a reusable convolution plan keyed by dimensions, kernel,
    // and scale-aware mode so repeated previews reuse contribution tables.
    // TODO(perf): Reuse contribution Vec storage through thread-local scratch so
    // repeated resizes with changing dimensions avoid allocator churn even when a
    // cached plan misses. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Reuse a horizontal/vertical scratch buffer for the future
    // separable path instead of allocating an intermediate image per resize.
    // Benchmark with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
    // before accepting.
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let x_contributions = prepare_axis_contributions(
        source_dimensions.width(),
        output_dimensions.width(),
        kernel,
        scale_aware,
    )?;
    let y_contributions = prepare_axis_contributions(
        source_dimensions.height(),
        output_dimensions.height(),
        kernel,
        scale_aware,
    )?;

    // TODO(perf): Split edge and interior rows/columns. Interior convolution can
    // skip clamping/duplicate source taps and use fixed tap counts for bicubic,
    // Lanczos2, and Lanczos3.
    // TODO(perf): Process output rows with `chunks_exact_mut` and carry a row
    // base offset to avoid recomputing `pixel_byte_offset` for every pixel.
    // Benchmark with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
    // before accepting.
    // TODO(perf): Add row-band tiling for convolution filters once the scalar hot
    // loop is stable; output rows are independent and should parallelize like
    // nearest/area tiling. Benchmark with `pnpm bench:resize:bicubic:tiling`
    // before accepting.
    for (output_y, y_contribution) in y_contributions.iter().enumerate() {
        // TODO(perf): Precompute y source row byte offsets once per y
        // contribution so every output pixel in the row reuses them. Benchmark
        // with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
        // before accepting.
        for (output_x, x_contribution) in x_contributions.iter().enumerate() {
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);

            // TODO(perf): Accumulate RGBA channels together for each source tap
            // rather than walking the same tap footprint once per channel.
            // TODO(perf): Precompute x*y weight products per output pixel or per
            // recurring contribution pair so the four RGBA channels do not all
            // repeat the same floating-point multiply chain. Benchmark with
            // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
            // before accepting.
            for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                output_rgba[output_offset + channel] = convolution_channel(
                    source_rgba,
                    source_width,
                    x_contribution,
                    y_contribution,
                    channel,
                );
            }
        }
    }

    Ok(())
}

fn prepare_axis_contributions<K: Kernel>(
    source_size: u32,
    output_size: u32,
    kernel: K,
    scale_aware: bool,
) -> Result<Vec<AxisContributions>, ProcessingError> {
    let source_len = usize::try_from(source_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "convolution source axis",
    })?;
    let output_len = usize::try_from(output_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "convolution output axis",
    })?;
    let scale = f64::from(output_size) / f64::from(source_size);
    // TODO(perf): Specialize scale-aware downscales and normal upscales at the
    // caller so this branch is not part of contribution planning for every axis.
    let filter_scale = if scale_aware && scale < 1.0 {
        scale
    } else {
        1.0
    };
    let radius = kernel.radius() / filter_scale;
    let mut contributions = Vec::with_capacity(output_len);

    // TODO(perf): Detect identity-axis contribution plans and return a compact
    // copy/alias representation rather than one single-tap Vec per coordinate.
    // Benchmark with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
    // before accepting.
    // TODO(perf): Detect exact integer ratios where contribution patterns repeat
    // periodically and clone a short pattern table instead of evaluating every
    // output coordinate. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Specialize non-scale-aware upscales separately from
    // scale-aware downscales so compact fixed-radius kernels avoid minification
    // planning overhead. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Plan contributions in f32 for scalar production filters if
    // byte-exact tests still pass; narrower weights may improve cache and SIMD
    // throughput. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Use incremental center updates instead of recomputing from a
    // division for every output coordinate.
    for output_coordinate in 0..output_len {
        let center = (output_coordinate as f64 + 0.5) / scale - 0.5;
        let first_source = (center - radius).floor() as isize;
        let last_source = (center + radius).ceil() as isize;
        // TODO(perf): Split contribution planning into edge and interior ranges;
        // interior coordinates can skip source-coordinate clamp calls entirely.
        // Benchmark with `pnpm bench:resize:bicubic` and
        // `pnpm bench:resize:lanczos3` before accepting.
        // TODO(perf): Reserve exact tap capacity from radius/support so each
        // contribution Vec avoids growth checks.
        let mut samples = Vec::new();
        let mut total_weight = 0.0;

        // TODO(perf): Collapse duplicate clamped edge taps into one weighted tap
        // during contribution preparation instead of sampling the same pixel
        // multiple times near borders.
        for source_coordinate in first_source..=last_source {
            let clamped_source = source_coordinate.clamp(0, source_len as isize - 1) as usize;
            let weight = kernel.weight((center - source_coordinate as f64) * filter_scale);

            if weight.abs() <= f64::EPSILON {
                continue;
            }

            samples.push(WeightedSourceIndex {
                index: clamped_source,
                weight,
            });
            total_weight += weight;
        }

        // TODO(perf): Pre-normalize with reciprocal multiply and consider
        // dropping zero/near-zero taps after normalization for Lanczos tails.
        // TODO(perf): Store the reciprocal total weight once and multiply samples
        // by it instead of dividing each sample during normalization. Benchmark
        // with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
        // before accepting.
        if total_weight.abs() > f64::EPSILON {
            for sample in &mut samples {
                sample.weight /= total_weight;
            }
        }

        contributions.push(AxisContributions { samples });
    }

    Ok(contributions)
}

// TODO(perf): Add filter-specific hot loops for fixed tap counts. Bicubic is 4x4,
// Lanczos2 is up to 4x4, and Lanczos3 is up to 6x6 for non-scale-aware paths.
// TODO(perf): Add a Wasm SIMD/native SIMD convolution kernel after fixed-tap
// loops exist, using vector loads for RGBA lanes and/or multiple pixels.
// Benchmark with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
// before accepting.
fn convolution_channel(
    source_rgba: &[u8],
    source_width: usize,
    x_contribution: &AxisContributions,
    y_contribution: &AxisContributions,
    channel: usize,
) -> u8 {
    let mut value = 0.0;

    // TODO(perf): In a two-pass implementation, horizontal pass can read source
    // rows sequentially and vertical pass can read scratch rows sequentially,
    // improving cache locality versus this nested random-access footprint.
    for y_sample in &y_contribution.samples {
        // TODO(perf): Hoist each y sample's source row slice or row byte offset
        // outside the inner x loop to avoid multiplying by source width for
        // every tap. Benchmark with `pnpm bench:resize:bicubic` and
        // `pnpm bench:resize:lanczos3` before accepting.
        for x_sample in &x_contribution.samples {
            let source_offset =
                rgba::pixel_byte_offset(source_width, x_sample.index, y_sample.index);
            value +=
                f64::from(source_rgba[source_offset + channel]) * x_sample.weight * y_sample.weight;
        }
    }

    value.round().clamp(0.0, 255.0) as u8
}
