use crate::{error::ProcessingError, image::rgba};

/// Default cap for row-band CPU tiling.
///
/// We intentionally use at most half the available native parallelism and cap it
/// at four workers so preview/export work does not monopolize the machine.
pub const DEFAULT_MAX_ROW_BAND_WORKERS: usize = 4;

/// Configuration for splitting output rows into CPU work bands.
#[derive(Debug, Clone, Copy)]
pub struct RowBandTiling {
    /// Minimum output pixels before row-band splitting can use more than one band.
    pub min_parallel_output_pixels: usize,
    /// Minimum rows per band so scheduling overhead does not dominate small images.
    pub min_rows_per_band: usize,
    /// Maximum workers to use for a single resize.
    pub max_workers: usize,
}

impl RowBandTiling {
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

/// A contiguous output-row range assigned to one worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowBand {
    pub output_y_start: usize,
    pub output_y_end: usize,
    pub output_byte_start: usize,
    pub output_byte_end: usize,
}

/// Runs a row-range kernel over output row bands.
///
/// Kernels receive absolute output row coordinates plus a mutable slice for only
/// that band's output rows. They must use `output_y_start..output_y_end` for
/// sampling so tiled and scalar execution keep identical pixel-center mapping.
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

    let useful_bands = output_height
        .div_ceil(config.min_rows_per_band.max(1))
        .max(1);
    default_worker_count(config.max_workers).min(useful_bands)
}

fn default_worker_count(max_workers: usize) -> usize {
    if max_workers == 0 {
        return 1;
    }

    #[cfg(all(feature = "tiling", not(target_arch = "wasm32")))]
    {
        std::thread::available_parallelism()
            .map(|parallelism| (parallelism.get() / 2).max(1).min(max_workers))
            .unwrap_or(1)
    }

    #[cfg(not(all(feature = "tiling", not(target_arch = "wasm32"))))]
    {
        let _ = max_workers;
        1
    }
}

#[cfg(all(feature = "tiling", not(target_arch = "wasm32")))]
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

#[cfg(not(all(feature = "tiling", not(target_arch = "wasm32"))))]
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
    use std::sync::Mutex;

    use super::{process_row_bands, RowBand, RowBandTiling};

    #[test]
    fn serial_fallback_processes_full_output_as_one_band() {
        let mut output = vec![0; 4 * 4 * 3];
        let config = RowBandTiling::new(usize::MAX, 2, 4);
        let observed_band = Mutex::new(None);

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
