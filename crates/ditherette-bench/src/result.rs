//! Serializable benchmark artifact types, result rows, and sample statistics.

use std::collections::BTreeMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub(crate) const ARTIFACT_SCHEMA: &str = "ditherette-bench-artifact";
pub(crate) const ARTIFACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchRun {
    pub(crate) schema: String,
    pub(crate) schema_version: u32,
    pub(crate) artifact_kind: String,
    pub(crate) run_id: String,
    pub(crate) created_at_unix: u64,
    pub(crate) tool: ToolInfo,
    pub(crate) command: String,
    pub(crate) domain: String,
    pub(crate) cli: Vec<String>,
    pub(crate) git: GitInfo,
    pub(crate) host: HostInfo,
    pub(crate) profile: String,
    pub(crate) measurement: Option<MeasurementArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) baseline: Option<BaselineArtifact>,
    pub(crate) results: Vec<BenchResult>,
}

impl BenchRun {
    pub(crate) fn new(
        command: &str,
        domain: &str,
        measurement: Option<MeasurementArtifact>,
        results: Vec<BenchResult>,
    ) -> Self {
        let created_at_unix = unix_seconds();
        Self {
            schema: ARTIFACT_SCHEMA.to_owned(),
            schema_version: ARTIFACT_SCHEMA_VERSION,
            artifact_kind: "run".to_owned(),
            run_id: run_id(created_at_unix),
            created_at_unix,
            tool: ToolInfo::current(),
            command: command.to_owned(),
            domain: domain.to_owned(),
            cli: std::env::args().collect(),
            git: GitInfo::current(),
            host: HostInfo::current(),
            profile: if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            }
            .to_owned(),
            measurement,
            baseline: None,
            results,
        }
    }

    pub(crate) fn as_baseline(&self, role: &str, name: &str) -> Self {
        let mut baseline = self.clone();
        baseline.artifact_kind = "baseline".to_owned();
        baseline.baseline = Some(BaselineArtifact {
            name: name.to_owned(),
            role: role.to_owned(),
            source_run_id: self.run_id.clone(),
        });
        baseline
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolInfo {
    pub(crate) name: String,
    pub(crate) version: String,
}

impl ToolInfo {
    fn current() -> Self {
        Self {
            name: "ditherette-bench".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GitInfo {
    pub(crate) commit: String,
    pub(crate) branch: String,
    pub(crate) dirty: bool,
}

impl GitInfo {
    fn current() -> Self {
        Self {
            commit: git_output(["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_owned()),
            branch: git_output(["branch", "--show-current"])
                .filter(|branch| !branch.is_empty())
                .unwrap_or_else(|| "unknown".to_owned()),
            dirty: git_dirty().unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HostInfo {
    pub(crate) os: String,
    pub(crate) arch: String,
    pub(crate) logical_cpus: usize,
}

impl HostInfo {
    fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            logical_cpus: std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct MeasurementArtifact {
    pub(crate) sample_size: usize,
    pub(crate) measurement_time_ms: u128,
    pub(crate) warmup_iterations: Option<usize>,
    pub(crate) warmup_time_ms: u128,
    pub(crate) target_sample_time_ms: u128,
    #[serde(default = "default_sample_mode")]
    pub(crate) sample_mode: String,
    #[serde(default = "default_cache_state")]
    pub(crate) cache_state: String,
    #[serde(default)]
    pub(crate) cache_scrub_size: usize,
    #[serde(default)]
    pub(crate) inter_sample_delay_ms: u128,
    #[serde(default)]
    pub(crate) live_stats: bool,
    #[serde(default)]
    pub(crate) preheat_time_ms: u128,
    #[serde(default = "default_process_priority")]
    pub(crate) process_priority: String,
}

fn default_process_priority() -> String {
    "normal".to_owned()
}

fn default_sample_mode() -> String {
    "throughput".to_owned()
}

fn default_cache_state() -> String {
    "warm".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BaselineArtifact {
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) source_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BenchResult {
    pub(crate) subject: String,
    pub(crate) case_id: String,
    pub(crate) fixture: String,
    pub(crate) fixture_kind: String,
    pub(crate) fixture_fingerprint: String,
    pub(crate) filter: String,
    pub(crate) variant: String,
    pub(crate) source_width: u32,
    pub(crate) source_height: u32,
    pub(crate) output_width: u32,
    pub(crate) output_height: u32,
    pub(crate) scale: f64,
    pub(crate) pixel_format: String,
    pub(crate) params_fingerprint: String,
    pub(crate) verified: bool,
    pub(crate) verification: Option<VerificationReport>,
    pub(crate) checksum: String,
    pub(crate) samples: usize,
    pub(crate) sample_ns: Vec<f64>,
    pub(crate) iterations_per_sample: usize,
    #[serde(default)]
    pub(crate) total_iterations: usize,
    pub(crate) min_ns: f64,
    pub(crate) median_ns: f64,
    pub(crate) mean_ns: f64,
    #[serde(default)]
    pub(crate) stdev_ns: f64,
    #[serde(default)]
    pub(crate) mode_ns: f64,
    pub(crate) p75_ns: f64,
    #[serde(default)]
    pub(crate) p90_ns: f64,
    pub(crate) p95_ns: f64,
    #[serde(default)]
    pub(crate) p99_ns: f64,
    pub(crate) max_ns: f64,
    pub(crate) output_mpix_per_s: f64,
    pub(crate) comparisons: BTreeMap<String, ComparisonReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VerificationReport {
    pub(crate) mode: String,
    pub(crate) passed: bool,
    pub(crate) first_mismatch: Option<MismatchReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MismatchReport {
    pub(crate) index: usize,
    pub(crate) left: u8,
    pub(crate) right: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ComparisonReport {
    pub(crate) baseline: String,
    pub(crate) median_ns: f64,
    pub(crate) ratio: f64,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SampleStats {
    pub(crate) min_ns: f64,
    pub(crate) median_ns: f64,
    pub(crate) mean_ns: f64,
    pub(crate) stdev_ns: f64,
    pub(crate) mode_ns: f64,
    pub(crate) p75_ns: f64,
    pub(crate) p90_ns: f64,
    pub(crate) p95_ns: f64,
    pub(crate) p99_ns: f64,
    pub(crate) max_ns: f64,
}

impl SampleStats {
    pub(crate) fn from_samples(samples: &[f64]) -> Self {
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
        let variance = sorted
            .iter()
            .map(|sample| {
                let delta = sample - mean;
                delta * delta
            })
            .sum::<f64>()
            / sorted.len() as f64;
        Self {
            min_ns: sorted[0],
            median_ns: percentile(&sorted, 50.0),
            mean_ns: mean,
            stdev_ns: variance.sqrt(),
            mode_ns: mode(&sorted),
            p75_ns: percentile(&sorted, 75.0),
            p90_ns: percentile(&sorted, 90.0),
            p95_ns: percentile(&sorted, 95.0),
            p99_ns: percentile(&sorted, 99.0),
            max_ns: sorted[sorted.len() - 1],
        }
    }
}

pub(crate) fn verify_exact(left: &[u8], right: &[u8]) -> VerificationReport {
    let first_mismatch = left
        .iter()
        .zip(right)
        .position(|(left, right)| left != right)
        .map(|index| MismatchReport {
            index,
            left: left[index],
            right: right[index],
        });
    VerificationReport {
        mode: "exact".to_owned(),
        passed: first_mismatch.is_none() && left.len() == right.len(),
        first_mismatch,
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn run_id(created_at_unix: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{created_at_unix}-{nanos:09}")
}

fn git_output<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_dirty() -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| !String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn mode(sorted: &[f64]) -> f64 {
    let mut best_value = sorted[0].round() as i64;
    let mut best_count = 0usize;
    let mut current_value = best_value;
    let mut current_count = 0usize;

    for sample in sorted {
        let value = sample.round() as i64;
        if value == current_value {
            current_count += 1;
        } else {
            if current_count > best_count {
                best_value = current_value;
                best_count = current_count;
            }
            current_value = value;
            current_count = 1;
        }
    }

    if current_count > best_count {
        best_value = current_value;
    }

    best_value as f64
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = percentile / 100.0 * (sorted.len() - 1) as f64;
    let low = rank.floor() as usize;
    let high = rank.ceil() as usize;
    let weight = rank - low as f64;
    sorted[low] * (1.0 - weight) + sorted[high] * weight
}
