use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        scalar::convolution::{resize_with_convolution, resize_with_convolution_into},
        shared::bicubic::Bicubic,
    },
};

#[allow(dead_code)]
pub(crate) fn resize_rgba_bicubic(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution(
        source_rgba,
        source_dimensions,
        output_dimensions,
        Bicubic,
        false,
    )
}

pub(crate) fn resize_rgba_bicubic_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Add a bicubic-specific separable implementation that mirrors
    // the optimized bilinear row-streaming path; a horizontal scratch row plus a
    // vertical pass may reduce the current x*y tap work. Benchmark with
    // `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Precompute four source indices and four normalized weights per
    // output coordinate in fixed-size arrays. Bicubic has a constant Catmull-Rom
    // footprint, so per-coordinate Vec allocations can move out of the hot path.
    // Benchmark with `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Store x-axis source byte offsets and y-axis row byte offsets in
    // the bicubic plan to avoid repeated `pixel_byte_offset` multiplication in
    // every 4x4 sample footprint. Benchmark with `pnpm bench:resize:bicubic`
    // before accepting.
    // TODO(perf): Accumulate RGBA lanes together per source tap in a dedicated
    // bicubic loop to avoid walking the same 4x4 footprint once per channel.
    // Benchmark with `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Split unclamped interior output pixels from edge pixels so the
    // common interior path can skip clamping and duplicate-edge handling.
    // Benchmark with `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Collapse duplicate clamped edge taps while planning bicubic
    // contributions to avoid sampling border pixels multiple times. Benchmark
    // with `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Reuse a thread-local bicubic row scratch and contribution plan
    // for repeated preview resizes with the same dimensions. Benchmark with
    // `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Add same-width and same-height one-axis bicubic paths so pure
    // vertical or horizontal resizes skip the unnecessary second axis. Benchmark
    // with `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Specialize exact integer-ratio downscales where Catmull-Rom
    // tap patterns repeat periodically, cloning a short contribution pattern
    // instead of evaluating every output coordinate. Benchmark with
    // `pnpm bench:resize:bicubic` before accepting.
    // TODO(perf): Convert normalized bicubic weights to fixed-point i16/i32 once
    // byte-exact expectations are settled; integer multiply/accumulate may beat
    // f64 in the fixed 4x4 path. Benchmark with `pnpm bench:resize:bicubic`
    // before accepting.
    // TODO(perf): Add a SIMD/Wasm-SIMD bicubic kernel for the fixed 4x4 RGBA
    // footprint after the scalar fixed-tap path exists. Benchmark with
    // `pnpm bench:resize:bicubic` before accepting.
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Bicubic,
        false,
    )
}
