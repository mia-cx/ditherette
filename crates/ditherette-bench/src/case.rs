//! Benchmark case expansion helpers.

use crate::{cli::Flags, error::BenchError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResizeScale {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl ResizeScale {
    pub(crate) fn uniform(scale: f64) -> Self {
        Self { x: scale, y: scale }
    }

    pub(crate) fn label(self) -> String {
        if self.is_uniform() {
            return format!("{}x", self.x);
        }
        format!("{}x-{}y", self.x, self.y)
    }

    pub(crate) fn is_uniform(self) -> bool {
        (self.x - self.y).abs() < f64::EPSILON
    }
}

pub(crate) fn scales_from_flags(flags: &Flags) -> Result<Vec<ResizeScale>, BenchError> {
    let mut scales = Vec::new();
    if let Some(values) = flags.optional("--scales") {
        for value in values.split(',').filter(|value| !value.is_empty()) {
            scales.push(ResizeScale::uniform(parse_scale(value)?));
        }
        return Ok(sorted_scales(scales));
    }
    if let Some(groups) = flags.optional("--scale-group") {
        for group in groups.split(',').filter(|group| !group.is_empty()) {
            scales.extend(scale_group(group)?);
        }
    }
    if let Some(pairs) = flags.optional("--scale-pairs") {
        for pair in pairs.split(',').filter(|pair| !pair.is_empty()) {
            scales.push(parse_scale_pair(pair)?);
        }
    }
    if scales.is_empty() {
        scales.push(ResizeScale::uniform(0.5));
    }
    Ok(sorted_scales(scales))
}

fn sorted_scales(mut scales: Vec<ResizeScale>) -> Vec<ResizeScale> {
    scales.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
    });
    scales.dedup_by(|left, right| {
        (left.x - right.x).abs() < f64::EPSILON && (left.y - right.y).abs() < f64::EPSILON
    });
    scales
}

fn parse_scale(value: &str) -> Result<f64, BenchError> {
    let scale = value
        .parse::<f64>()
        .map_err(|error| BenchError::Config(format!("invalid scale {value:?}: {error}")))?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(BenchError::Config(format!(
            "invalid scale {value:?}; expected a finite value greater than zero"
        )));
    }
    Ok(scale)
}

fn parse_scale_pair(value: &str) -> Result<ResizeScale, BenchError> {
    let Some((x, y)) = value.split_once('x') else {
        return Err(BenchError::Config(format!(
            "invalid scale pair {value:?}; expected SCALE_XxSCALE_Y"
        )));
    };
    Ok(ResizeScale {
        x: parse_scale(x)?,
        y: parse_scale(y)?,
    })
}

fn scale_group(group: &str) -> Result<Vec<ResizeScale>, BenchError> {
    let scales = match group {
        "preview" => uniform(vec![0.5, 0.25]),
        "quick" | "resize-critical" => uniform(vec![2.0, 0.95, 0.75, 0.5, 0.25, 0.125]),
        "near-identity" => uniform(vec![0.95, 0.97, 0.98, 0.99, 1.0, 1.01, 1.02, 1.03, 1.05]),
        "upscale" => uniform(vec![
            1.01, 1.02, 1.03, 1.05, 1.07, 1.1, 1.15, 1.25, 1.5, 1.67, 2.0, 2.38, 2.83, 3.36, 4.0,
            4.76, 5.66, 6.73, 8.0,
        ]),
        "downscale" => uniform(vec![
            0.99, 0.98, 0.97, 0.95, 0.93, 0.9, 0.85, 0.75, 0.5, 0.33, 0.25, 0.19, 0.16, 0.125, 0.1,
        ]),
        "identity" => uniform(vec![1.0]),
        "nearest-anisotropic"
        | "area-anisotropic"
        | "bilinear-anisotropic"
        | "convolution-anisotropic" => vec![
            ResizeScale { x: 0.5, y: 1.0 },
            ResizeScale { x: 1.0, y: 0.5 },
            ResizeScale { x: 0.75, y: 1.0 },
            ResizeScale { x: 1.0, y: 0.75 },
            ResizeScale { x: 1.5, y: 1.0 },
            ResizeScale { x: 1.0, y: 1.5 },
        ],
        "bilinear-prior-art" => uniform(vec![0.875, 1.8]),
        "partial" => uniform(vec![
            0.1, 0.125, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.05, 1.5, 2.0,
        ]),
        "full" => uniform(vec![
            0.1, 0.125, 0.16, 0.19, 0.25, 0.33, 0.5, 0.75, 0.85, 0.9, 0.93, 0.95, 0.97, 0.98, 0.99,
            1.0, 1.01, 1.02, 1.03, 1.05, 1.07, 1.1, 1.15, 1.25, 1.5, 1.67, 2.0, 2.38, 2.83, 3.36,
            4.0, 4.76, 5.66, 6.73, 8.0,
        ]),
        "stress" => uniform(vec![4.0, 2.0, 0.5, 0.125]),
        other => return Err(BenchError::Config(format!("unknown scale group {other:?}"))),
    };
    Ok(scales)
}

fn uniform(scales: Vec<f64>) -> Vec<ResizeScale> {
    scales.into_iter().map(ResizeScale::uniform).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_reject_invalid_values() {
        for value in ["NaN", "inf", "-inf", "0", "-1", "1e309"] {
            let flags = Flags::parse(&["--scales".into(), value.into()]).unwrap();
            let error = scales_from_flags(&flags).expect_err(value);
            assert!(matches!(error, BenchError::Config(_)));
            assert_eq!(
                error.to_string(),
                format!("invalid scale {value:?}; expected a finite value greater than zero")
            );
        }
    }

    #[test]
    fn scale_pairs_reject_invalid_values() {
        for value in ["NaN", "inf", "-inf", "0", "-1", "1e309"] {
            for pair in [format!("{value}x1"), format!("1x{value}")] {
                let flags = Flags::parse(&["--scale-pairs".into(), pair.clone()]).unwrap();
                let error = scales_from_flags(&flags).expect_err(&pair);
                assert!(matches!(error, BenchError::Config(_)));
                assert_eq!(
                    error.to_string(),
                    format!("invalid scale {value:?}; expected a finite value greater than zero")
                );
            }
        }
    }

    #[test]
    fn scales_preserve_finite_positive_values() {
        let flags = Flags::parse(&["--scales".into(), "2,0.5,1,0.5".into()]).unwrap();
        assert_eq!(
            scales_from_flags(&flags).unwrap(),
            vec![
                ResizeScale::uniform(0.5),
                ResizeScale::uniform(1.0),
                ResizeScale::uniform(2.0),
            ]
        );
        let flags = Flags::parse(&["--scale-pairs".into(), "2x0.5,0.5x1".into()]).unwrap();
        assert_eq!(
            scales_from_flags(&flags).unwrap(),
            vec![
                ResizeScale { x: 0.5, y: 1.0 },
                ResizeScale { x: 2.0, y: 0.5 }
            ]
        );
    }
}
