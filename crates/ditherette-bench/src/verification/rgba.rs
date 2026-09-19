//! RGBA comparison shared by resize measurements and typed conformance.

use ditherette_bench_api::verification::Digest256;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub mode: String,
    pub passed: bool,
    /// Numeric diagnostics never establish approval for non-exact output.
    #[serde(default)]
    pub within_bounds: bool,
    #[serde(default)]
    pub candidate_digest: Option<Digest256>,
    pub first_mismatch: Option<MismatchReport>,
    #[serde(default)]
    pub bytes: usize,
    #[serde(default)]
    pub pixels: usize,
    #[serde(default)]
    pub differing_bytes: usize,
    #[serde(default)]
    pub differing_pixels: usize,
    #[serde(default)]
    pub max_abs_diff: u8,
    #[serde(default)]
    pub mean_abs_diff: f64,
    #[serde(default)]
    pub max_color_distance: f64,
    #[serde(default)]
    pub mean_color_distance: f64,
    #[serde(default)]
    pub rms_color_distance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MismatchReport {
    pub index: usize,
    #[serde(default)]
    pub pixel_index: usize,
    pub left: u8,
    pub right: u8,
    #[serde(default)]
    pub left_rgba: [u8; 4],
    #[serde(default)]
    pub right_rgba: [u8; 4],
    #[serde(default)]
    pub color_distance: f64,
}

impl VerificationReport {
    /// Require current, complete exact evidence when promoting persisted results.
    pub fn is_exact(&self) -> bool {
        self.passed
            && self.within_bounds
            && self.bytes > 0
            && self.bytes % 4 == 0
            && self.pixels == self.bytes / 4
            && self.differing_bytes == 0
            && self.differing_pixels == 0
            && self.first_mismatch.is_none()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VerificationBounds {
    pub max_color_distance: f64,
    pub max_mean_color_distance: f64,
    pub max_rms_color_distance: f64,
}

impl VerificationBounds {
    pub fn exact() -> Self {
        Self {
            max_color_distance: 0.0,
            max_mean_color_distance: 0.0,
            max_rms_color_distance: 0.0,
        }
    }

    pub fn bounded_default() -> Self {
        Self {
            max_color_distance: 2.0,
            max_mean_color_distance: 2.0,
            max_rms_color_distance: 2.0,
        }
    }

    pub fn is_exact(self) -> bool {
        self.max_color_distance == 0.0
            && self.max_mean_color_distance == 0.0
            && self.max_rms_color_distance == 0.0
    }

    fn mode(self) -> String {
        if self.is_exact() {
            return "exact".to_owned();
        }

        format!(
            "bounded(max_color_distance<={:.3}, mean<={:.3}, rms<={:.3})",
            self.max_color_distance, self.max_mean_color_distance, self.max_rms_color_distance
        )
    }
}

pub fn verify_with_bounds(
    left: &[u8],
    right: &[u8],
    bounds: VerificationBounds,
) -> VerificationReport {
    let bytes = left.len().max(right.len());
    let pixels = bytes.div_ceil(4);
    let mut first_mismatch = None;
    let mut differing_bytes = left.len().abs_diff(right.len());
    let mut differing_pixels = if left.len() == right.len() { 0 } else { pixels };
    let mut max_abs_diff = if left.len() == right.len() {
        0
    } else {
        u8::MAX
    };
    let mut byte_diff_sum = 0usize;
    let mut max_color_distance = if left.len() == right.len() {
        0.0
    } else {
        f64::MAX
    };
    let mut color_distance_sum = 0.0;
    let mut color_distance_squared_sum = 0.0;

    for (pixel_index, (left_pixel, right_pixel)) in
        left.chunks_exact(4).zip(right.chunks_exact(4)).enumerate()
    {
        let mut channel_squared_sum = 0.0;
        let mut pixel_differs = false;

        for channel in 0..4 {
            let left_byte = left_pixel[channel];
            let right_byte = right_pixel[channel];
            let abs_diff = left_byte.abs_diff(right_byte);
            if abs_diff == 0 {
                continue;
            }

            let byte_index = pixel_index * 4 + channel;
            first_mismatch.get_or_insert_with(|| MismatchReport {
                index: byte_index,
                pixel_index,
                left: left_byte,
                right: right_byte,
                left_rgba: left_pixel.try_into().expect("chunk has four channels"),
                right_rgba: right_pixel.try_into().expect("chunk has four channels"),
                color_distance: color_distance(left_pixel, right_pixel),
            });
            differing_bytes += 1;
            max_abs_diff = max_abs_diff.max(abs_diff);
            byte_diff_sum += usize::from(abs_diff);
            channel_squared_sum += f64::from(abs_diff).powi(2);
            pixel_differs = true;
        }

        let pixel_color_distance = channel_squared_sum.sqrt();
        if pixel_differs {
            differing_pixels += 1;
            max_color_distance = f64::max(max_color_distance, pixel_color_distance);
        }
        color_distance_sum += pixel_color_distance;
        color_distance_squared_sum += pixel_color_distance.powi(2);
    }

    let mean_color_distance = if pixels == 0 {
        0.0
    } else {
        color_distance_sum / pixels as f64
    };
    let rms_color_distance = if pixels == 0 {
        0.0
    } else {
        (color_distance_squared_sum / pixels as f64).sqrt()
    };
    let within_bounds = left.len() == right.len()
        && left.len() % 4 == 0
        && max_color_distance <= bounds.max_color_distance
        && mean_color_distance <= bounds.max_mean_color_distance
        && rms_color_distance <= bounds.max_rms_color_distance;

    VerificationReport {
        mode: bounds.mode(),
        passed: within_bounds && bytes > 0 && differing_bytes == 0,
        within_bounds,
        candidate_digest: Some(super::content_digest(right)),
        first_mismatch,
        bytes,
        pixels,
        differing_bytes,
        differing_pixels,
        max_abs_diff,
        mean_abs_diff: if bytes == 0 {
            0.0
        } else {
            byte_diff_sum as f64 / bytes as f64
        },
        max_color_distance,
        mean_color_distance,
        rms_color_distance,
    }
}

fn color_distance(left: &[u8], right: &[u8]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| f64::from(left.abs_diff(right)).powi(2))
        .sum::<f64>()
        .sqrt()
}
