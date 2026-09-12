//! Fresh paired evidence. Historical samples never replace a required role.

pub mod coordinator;

use ditherette_bench_api::verification::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::verification::{
    verify_three_way, ThreeWayReport, VerificationBounds, VerificationStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Accepted,
    Candidate,
}

/// Application cache state is independent of CPU-cache scrubbing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationCache {
    NotApplicable,
    Cold,
    Warm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SampleMode {
    SingleCall,
    Throughput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CallScope {
    NativeKernel,
    CompleteCall,
    Initialization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Measurement {
    pub mode: SampleMode,
    pub scope: CallScope,
    pub application_cache: ApplicationCache,
    pub samples: usize,
    pub measurement_ms: u64,
    pub warmup_ms: u64,
    pub target_sample_ms: u64,
}

/// Native fixture requests currently use the existing center/default resize recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairCase {
    pub name: String,
    pub identity: CaseIdentity,
    pub source: Dimensions,
    pub rgba: Vec<u8>,
    pub reference_subject: String,
    pub accepted_subject: String,
    pub candidate_subject: String,
    pub measurement: Measurement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    pub label: String,
    pub reference_state: ReferenceState,
    /// Even number of pairs, at least two. The budget is fixed before running.
    pub pairs: usize,
    pub host_load_notes: String,
    pub cases: Vec<PairCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildIdentity {
    pub revision: String,
    pub dirty: bool,
    pub rustc: String,
    pub tool_version: String,
    pub configuration: String,
    pub recorded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub kernel: String,
    pub cpu: String,
    pub logical_cpus: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Executable {
    pub path: PathBuf,
    pub identity: ArtifactIdentity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedPair {
    pub schema: String,
    pub experiment: Experiment,
    pub accepted: Executable,
    pub candidate: Executable,
    pub machine: Machine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialRequest {
    pub role: Role,
    pub pair: usize,
    pub reference_state: ReferenceState,
    pub executable: ArtifactIdentity,
    pub case: PairCase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub role: Role,
    pub pair: usize,
    pub case_name: String,
    pub build: BuildIdentity,
    pub measurement: Measurement,
    pub warmup_iterations: usize,
    pub warmup_elapsed_ns: u128,
    /// Per-call ns for each measured sample, without aggregation across modes.
    pub sample_ns: Vec<f64>,
    pub iterations_per_sample: usize,
    pub reference: RecordedOutput,
    pub output: RecordedOutput,
    pub pid: u32,
    pub max_live_benchmark_processes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Gate {
    Pass,
    Regression,
    Inconclusive,
    Incomplete,
    Incorrect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaseComparison {
    pub case_name: String,
    pub gate: Gate,
    pub issues: Vec<String>,
    pub pair_ratios: Vec<f64>,
    pub accepted_median_ns: Option<f64>,
    pub candidate_median_ns: Option<f64>,
    pub median_ratio: Option<f64>,
    pub verification: Vec<ThreeWayReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairReport {
    pub schema: String,
    pub gate: Gate,
    pub cases: Vec<CaseComparison>,
}

/// Compare fresh trial pairs only. No API in this module promotes a candidate.
pub fn compare(prepared: &PreparedPair, trials: &[TrialResult]) -> PairReport {
    let mut cases = Vec::new();
    for case in &prepared.experiment.cases {
        let mut result = CaseComparison {
            case_name: case.name.clone(),
            gate: Gate::Incomplete,
            issues: Vec::new(),
            pair_ratios: Vec::new(),
            accepted_median_ns: None,
            candidate_median_ns: None,
            median_ratio: None,
            verification: Vec::new(),
        };
        if prepared
            .accepted
            .identity
            .revision
            .eq_ignore_ascii_case(&prepared.candidate.identity.revision)
        {
            result
                .issues
                .push("paired benchmarks require distinct source revisions".into());
        }
        let mut accepted_samples = Vec::new();
        let mut candidate_samples = Vec::new();
        for pair in 0..prepared.experiment.pairs {
            let mut outputs = Vec::new();
            for role in [Role::Accepted, Role::Candidate] {
                let matches: Vec<_> = trials
                    .iter()
                    .filter(|trial| {
                        trial.case_name == case.name && trial.pair == pair && trial.role == role
                    })
                    .collect();
                if matches.len() != 1 {
                    result.issues.push(format!(
                        "pair {pair} {role:?} requires exactly one fresh result"
                    ));
                    continue;
                }
                let trial = matches[0];
                let executable = match role {
                    Role::Accepted => &prepared.accepted,
                    Role::Candidate => &prepared.candidate,
                };
                let subject = match role {
                    Role::Accepted => &case.accepted_subject,
                    Role::Candidate => &case.candidate_subject,
                };
                if trial.build.dirty
                    || trial.build.revision != executable.identity.revision
                    || trial.build.rustc.is_empty()
                    || trial.build.tool_version.is_empty()
                    || trial.build.configuration.is_empty()
                    || !trial.build.recorded
                    || trial.measurement != case.measurement
                    || trial.output.implementation.artifact != executable.identity
                    || trial.reference.implementation.artifact != executable.identity
                    || trial.output.implementation.subject != *subject
                    || trial.reference.implementation.subject != case.reference_subject
                    || trial.sample_ns.len() < 5
                    || trial.sample_ns.len() > case.measurement.samples
                    || trial
                        .sample_ns
                        .iter()
                        .any(|sample| !sample.is_finite() || *sample <= 0.0)
                    || trial.iterations_per_sample == 0
                    || trial.warmup_iterations == 0
                    || trial.warmup_elapsed_ns == 0
                    || trial.max_live_benchmark_processes != 1
                    || (case.measurement.mode == SampleMode::SingleCall
                        && trial.iterations_per_sample != 1)
                {
                    result.issues.push(format!(
                        "pair {pair} {role:?} has mismatched or incomplete evidence"
                    ));
                    continue;
                }
                outputs.push(trial);
            }
            if outputs.len() != 2 {
                continue;
            }
            let (accepted, candidate) = (outputs[0], outputs[1]);
            if accepted.build.rustc != candidate.build.rustc
                || accepted.build.tool_version != candidate.build.tool_version
                || accepted.build.configuration != candidate.build.configuration
            {
                result
                    .issues
                    .push(format!("pair {pair} uses different toolchains"));
            }
            // Both artifact-local references must agree with the same named semantics.
            for accepted_record in [&accepted.output, &candidate.reference] {
                result.verification.push(verify_three_way(
                    &case.identity,
                    &ThreeWayOutputs {
                        reference_state: prepared.experiment.reference_state,
                        reference: Some(accepted.reference.clone()),
                        accepted: Some(accepted_record.clone()),
                        candidate: Some(candidate.output.clone()),
                    },
                    VerificationBounds::exact(),
                ));
            }
            result
                .pair_ratios
                .push(median(&candidate.sample_ns) / median(&accepted.sample_ns));
            accepted_samples.extend_from_slice(&accepted.sample_ns);
            candidate_samples.extend_from_slice(&candidate.sample_ns);
        }
        if result
            .verification
            .iter()
            .any(|proof| proof.status != VerificationStatus::Exact)
        {
            result.gate = Gate::Incorrect;
        } else if !result.issues.is_empty()
            || result.pair_ratios.len() != prepared.experiment.pairs
            || prepared.experiment.pairs < 2
            || prepared.experiment.pairs % 2 != 0
        {
            result.gate = Gate::Incomplete;
        } else {
            let a = median(&accepted_samples);
            let b = median(&candidate_samples);
            let ratio = b / a;
            result.accepted_median_ns = Some(a);
            result.candidate_median_ns = Some(b);
            result.median_ratio = Some(ratio);
            // Each alternating order must independently confirm a threshold crossing.
            // Mixed or unstable pairs remain inconclusive within the fixed run budget.
            let min = result
                .pair_ratios
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let max = result.pair_ratios.iter().copied().fold(0.0, f64::max);
            result.gate = if ratio > 1.10 && min > 1.10 {
                Gate::Regression
            } else if ratio <= 1.10 && max <= 1.10 && max / min <= 1.10 {
                Gate::Pass
            } else {
                Gate::Inconclusive
            };
        }
        cases.push(result);
    }
    let extra = trials.iter().any(|trial| {
        trial.pair >= prepared.experiment.pairs
            || !prepared
                .experiment
                .cases
                .iter()
                .any(|case| case.name == trial.case_name)
    });
    let gate = if cases.iter().any(|case| case.gate == Gate::Incorrect) {
        Gate::Incorrect
    } else if cases.is_empty() || extra || cases.iter().any(|case| case.gate == Gate::Incomplete) {
        Gate::Incomplete
    } else if cases.iter().any(|case| case.gate == Gate::Regression) {
        Gate::Regression
    } else if cases.iter().any(|case| case.gate == Gate::Inconclusive) {
        Gate::Inconclusive
    } else {
        Gate::Pass
    };
    PairReport {
        schema: "ditherette-fresh-pair-v1".into(),
        gate,
        cases,
    }
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        sorted[middle - 1] / 2.0 + sorted[middle] / 2.0
    } else {
        sorted[middle]
    }
}
