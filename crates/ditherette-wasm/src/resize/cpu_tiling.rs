use crate::{error::ProcessingError, image::rgba};

/// Maximum CPU workers used by default for row-band resize work.
///
/// The cap keeps browser/native preview work from grabbing the whole machine;
/// callers can still provide a tighter cap through [`RowBandTiling`].
pub const DEFAULT_MAX_ROW_BAND_WORKERS: usize = 4;

/// Configuration for CPU row-band tiling.
#[derive(Debug, Clone, Copy)]
pub struct RowBandTiling {
    /// Minimum output pixels before parallel execution is worth considering.
    pub min_parallel_output_pixels: usize,
    /// Minimum rows assigned to each worker band.
    pub min_rows_per_band: usize,
    /// Upper bound on workers for one resize.
    pub max_workers: usize,
}

impl RowBandTiling {
    /// Creates a row-band tiling configuration.
    pub const fn new(
        min_parallel_output_pixels: usize,
        min_rows_per_band: usize,
        max_workers: usize,
    ) -> Self {
        Self {
            min_parallel_output_pixels,
            min_rows_per_band,
            max_workers,
        }
    }
}

/// A contiguous set of output rows owned by one CPU worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBand {
    pub output_y_start: usize,
    pub output_y_end: usize,
    pub output_byte_start: usize,
    pub output_byte_end: usize,
}

/// Applies a resize kernel over row bands, using parallel workers when the
/// platform/build supports them and the output is large enough.
///
/// The kernel receives absolute output row bounds plus a mutable slice covering
/// only that band's output rows. Filters must use `band.output_y_start` for
/// sampling coordinates so tiled and full-frame execution produce identical
/// pixels.
pub fn process_row_bands<F>(
    output_rgba: &mut [u8],
    output_width: usize,
    output_height: usize,
    config: RowBandTiling,
    kernel: F,
) -> Result<(), ProcessingError>
where
    F: Fn(RowBand, &mut [u8]) -> Result<(), ProcessingError> + Sync,
{
    let output_row_byte_len = output_width.checked_mul(rgba::RGBA_CHANNEL_COUNT).ok_or(
        ProcessingError::SizeOverflow {
            context: "row-band output row byte length",
        },
    )?;
    let band_count = effective_band_count(output_width, output_height, config);

    if band_count <= 1 {
        let band = RowBand {
            output_y_start: 0,
            output_y_end: output_height,
            output_byte_start: 0,
            output_byte_end: output_rgba.len(),
        };
        return kernel(band, output_rgba);
    }

    let band_height = output_height.div_ceil(band_count);

    process_chunks(
        output_rgba,
        output_row_byte_len,
        output_height,
        band_height,
        kernel,
    )
}

fn effective_band_count(output_width: usize, output_height: usize, config: RowBandTiling) -> usize {
    let output_pixels = output_width.saturating_mul(output_height);
    if output_pixels < config.min_parallel_output_pixels {
        return 1;
    }

    let min_rows_per_band = config.min_rows_per_band.max(1);
    let useful_bands = output_height.div_ceil(min_rows_per_band).max(1);
    default_worker_count(config.max_workers).min(useful_bands)
}

fn default_worker_count(max_workers: usize) -> usize {
    if max_workers == 0 {
        return 1;
    }

    #[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
    {
        std::thread::available_parallelism()
            .map(|parallelism| (parallelism.get() / 2).max(1).min(max_workers))
            .unwrap_or(1)
    }

    #[cfg(not(all(feature = "parallel", not(target_arch = "wasm32"))))]
    {
        let _ = max_workers;
        1
    }
}

#[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
fn process_chunks<F>(
    output_rgba: &mut [u8],
    output_row_byte_len: usize,
    output_height: usize,
    band_height: usize,
    kernel: F,
) -> Result<(), ProcessingError>
where
    F: Fn(RowBand, &mut [u8]) -> Result<(), ProcessingError> + Sync,
{
    use rayon::prelude::*;

    let band_byte_len =
        output_row_byte_len
            .checked_mul(band_height)
            .ok_or(ProcessingError::SizeOverflow {
                context: "row-band byte length",
            })?;

    output_rgba
        .par_chunks_mut(band_byte_len)
        .enumerate()
        .try_for_each(|(band_index, output_rows)| {
            let output_y_start = band_index * band_height;
            let output_y_end =
                (output_y_start + output_rows.len() / output_row_byte_len).min(output_height);
            let output_byte_start = output_y_start * output_row_byte_len;

            kernel(
                RowBand {
                    output_y_start,
                    output_y_end,
                    output_byte_start,
                    output_byte_end: output_byte_start + output_rows.len(),
                },
                output_rows,
            )
        })
}

#[cfg(not(all(feature = "parallel", not(target_arch = "wasm32"))))]
fn process_chunks<F>(
    output_rgba: &mut [u8],
    output_row_byte_len: usize,
    output_height: usize,
    band_height: usize,
    kernel: F,
) -> Result<(), ProcessingError>
where
    F: Fn(RowBand, &mut [u8]) -> Result<(), ProcessingError> + Sync,
{
    let band_byte_len =
        output_row_byte_len
            .checked_mul(band_height)
            .ok_or(ProcessingError::SizeOverflow {
                context: "row-band byte length",
            })?;

    for (band_index, output_rows) in output_rgba.chunks_mut(band_byte_len).enumerate() {
        let output_y_start = band_index * band_height;
        let output_y_end =
            (output_y_start + output_rows.len() / output_row_byte_len).min(output_height);
        let output_byte_start = output_y_start * output_row_byte_len;

        kernel(
            RowBand {
                output_y_start,
                output_y_end,
                output_byte_start,
                output_byte_end: output_byte_start + output_rows.len(),
            },
            output_rows,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{process_row_bands, RowBand, RowBandTiling};

    #[test]
    fn serial_fallback_processes_full_output_as_one_band() {
        let mut output = vec![0; 4 * 4 * 3];
        let config = RowBandTiling::new(usize::MAX, 2, 4);
        let observed_band = std::sync::Mutex::new(None);

        process_row_bands(&mut output, 4, 3, config, |band, output_rows| {
            *observed_band.lock().unwrap() = Some(band);
            output_rows.fill(7);
            Ok(())
        })
        .unwrap();

        assert_eq!(
            *observed_band.lock().unwrap(),
            Some(RowBand {
                output_y_start: 0,
                output_y_end: 3,
                output_byte_start: 0,
                output_byte_end: output.len(),
            })
        );
        assert!(output.iter().all(|byte| *byte == 7));
    }
}
