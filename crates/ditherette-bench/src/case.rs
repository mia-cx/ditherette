//! Benchmark case expansion helpers.

use crate::{cli::Flags, error::BenchError};

pub(crate) fn scales_from_flags(flags: &Flags) -> Result<Vec<f64>, BenchError> {
    let mut scales = Vec::new();
    if let Some(values) = flags.optional("--scales") {
        for value in values.split(',').filter(|value| !value.is_empty()) {
            scales.push(value.parse::<f64>().map_err(|error| {
                BenchError::Config(format!("invalid scale {value:?}: {error}"))
            })?);
        }
        scales.sort_by(f64::total_cmp);
        scales.dedup_by(|left, right| (*left - *right).abs() < f64::EPSILON);
        return Ok(scales);
    }
    if let Some(groups) = flags.optional("--scale-group") {
        for group in groups.split(',').filter(|group| !group.is_empty()) {
            scales.extend(scale_group(group)?);
        }
    }
    if scales.is_empty() {
        scales.push(0.5);
    }
    scales.sort_by(f64::total_cmp);
    scales.dedup_by(|left, right| (*left - *right).abs() < f64::EPSILON);
    Ok(scales)
}

fn scale_group(group: &str) -> Result<Vec<f64>, BenchError> {
    Ok(match group {
        "preview" => vec![0.5, 0.25],
        "quick" | "resize-critical" => vec![2.0, 0.95, 0.75, 0.5, 0.25, 0.125],
        "near-identity" => vec![0.95, 0.97, 0.98, 0.99, 1.0, 1.01, 1.02, 1.03, 1.05],
        "upscale" => vec![
            1.01, 1.02, 1.03, 1.05, 1.07, 1.1, 1.15, 1.25, 1.5, 1.67, 2.0, 2.38, 2.83, 3.36, 4.0,
            4.76, 5.66, 6.73, 8.0,
        ],
        "downscale" => vec![
            0.99, 0.98, 0.97, 0.95, 0.93, 0.9, 0.85, 0.75, 0.5, 0.33, 0.25, 0.19, 0.16, 0.13,
        ],
        "stress" => vec![4.0, 2.0, 0.5, 0.125],
        other => return Err(BenchError::Config(format!("unknown scale group {other:?}"))),
    })
}
