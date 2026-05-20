//! Result comparison and acceptance-gate helpers.

use ditherette_bench_api::SubjectId;

use crate::{
    cli::Flags,
    error::BenchError,
    measure::MeasurementConfig,
    result::{BenchResult, BenchRun, ComparisonReport},
};

pub(crate) fn ensure_compatible_baseline(
    baseline: &BenchRun,
    command: &str,
    domain: &str,
    measurement: &MeasurementConfig,
) -> Result<(), BenchError> {
    if baseline.command != command || baseline.domain != domain {
        return Err(BenchError::Baseline(format!(
            "baseline is for {}/{} but current run is {command}/{domain}",
            baseline.command, baseline.domain
        )));
    }
    if baseline.measurement.as_ref() != Some(&measurement.artifact()) {
        return Err(BenchError::Baseline(
            "baseline measurement config does not match the current run".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn require_accepted_comparisons(
    results: &[BenchResult],
    baseline: &BenchRun,
) -> Result<(), BenchError> {
    let missing = results
        .iter()
        .filter(|result| {
            !baseline
                .results
                .iter()
                .any(|baseline_result| exact_match(result, baseline_result))
        })
        .map(|result| format!("{} {}", result.subject, result.case_id))
        .collect::<Vec<_>>();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(BenchError::Baseline(format!(
            "accepted baseline is missing or incompatible for: {}",
            missing.join(", ")
        )))
    }
}

pub(crate) fn has_exact_comparisons(results: &[BenchResult], baseline: &BenchRun) -> bool {
    results.iter().all(|result| {
        baseline
            .results
            .iter()
            .any(|baseline_result| exact_match(result, baseline_result))
    })
}

pub(crate) fn attach_accepted_comparisons(
    results: &mut [BenchResult],
    baseline: Option<&BenchRun>,
) {
    let Some(baseline) = baseline else { return };
    attach_exact_baseline_comparisons(results, baseline, "accepted", "accepted");
}

pub(crate) fn attach_measured_spec_comparisons(
    results: &mut [BenchResult],
    spec_subject: &SubjectId,
) {
    attach_measured_subject_comparisons(results, spec_subject, "spec");
}

pub(crate) fn attach_measured_oracle_comparisons(
    results: &mut [BenchResult],
    oracle_subject: &SubjectId,
) {
    attach_measured_subject_comparisons(results, oracle_subject, "oracle");
}

fn attach_measured_subject_comparisons(
    results: &mut [BenchResult],
    subject_id: &SubjectId,
    key: &str,
) {
    let baseline_results = results
        .iter()
        .filter(|result| result.subject == subject_id.as_str())
        .cloned()
        .collect::<Vec<_>>();
    for result in results {
        if result.subject == subject_id.as_str() {
            continue;
        }
        if let Some(spec) = baseline_results
            .iter()
            .find(|candidate| semantic_match(result, candidate))
        {
            result.comparisons.insert(
                key.to_owned(),
                comparison(subject_id.as_str(), result.median_ns, spec.median_ns),
            );
        }
    }
}

pub(crate) fn attach_named_spec_comparisons(results: &mut [BenchResult], baseline: &BenchRun) {
    attach_semantic_baseline_comparisons(results, baseline, "spec", "spec");
}

pub(crate) fn attach_named_oracle_comparisons(results: &mut [BenchResult], baseline: &BenchRun) {
    let label = baseline
        .baseline
        .as_ref()
        .map(|baseline| baseline.name.as_str())
        .unwrap_or("oracle");
    attach_semantic_baseline_comparisons(results, baseline, "oracle", label);
}

pub(crate) fn attach_previous_comparisons(results: &mut [BenchResult], baseline: &BenchRun) {
    attach_exact_baseline_comparisons(results, baseline, "accepted", "previous");
}

fn attach_semantic_baseline_comparisons(
    results: &mut [BenchResult],
    baseline: &BenchRun,
    key: &str,
    label: &str,
) {
    for result in results {
        if let Some(spec) = baseline
            .results
            .iter()
            .find(|candidate| semantic_match(result, candidate))
        {
            result.comparisons.insert(
                key.to_owned(),
                comparison(label, result.median_ns, spec.median_ns),
            );
        }
    }
}

fn attach_exact_baseline_comparisons(
    results: &mut [BenchResult],
    baseline: &BenchRun,
    key: &str,
    label: &str,
) {
    for result in results {
        if let Some(baseline_result) = baseline
            .results
            .iter()
            .find(|candidate| exact_match(result, candidate))
        {
            result.comparisons.insert(
                key.to_owned(),
                comparison(label, result.median_ns, baseline_result.median_ns),
            );
        }
    }
}

pub(crate) fn attach_pair_comparisons(
    results: &mut [BenchResult],
    left: &SubjectId,
    right: &SubjectId,
) {
    let left_results = results
        .iter()
        .filter(|result| result.subject == left.as_str())
        .cloned()
        .collect::<Vec<_>>();
    for result in results {
        if result.subject != right.as_str() {
            continue;
        }
        if let Some(left_result) = left_results
            .iter()
            .find(|candidate| semantic_match(result, candidate))
        {
            result.comparisons.insert(
                "left".to_owned(),
                comparison(left.as_str(), result.median_ns, left_result.median_ns),
            );
        }
    }
}

pub(crate) struct AcceptanceReport {
    pub(crate) passed: bool,
    pub(crate) message: String,
}

pub(crate) fn acceptance_report(
    results: &[BenchResult],
    flags: &Flags,
) -> Result<AcceptanceReport, BenchError> {
    let Some(percent) = flags.optional("--fail-on-regression-percent") else {
        return Ok(AcceptanceReport {
            passed: true,
            message: String::new(),
        });
    };
    let percent = percent.parse::<f64>().map_err(|error| {
        BenchError::Config(format!(
            "invalid --fail-on-regression-percent value {percent:?}: {error}"
        ))
    })?;
    let max_ratio = 1.0 + percent / 100.0;
    let regressions = results
        .iter()
        .filter_map(|result| {
            let comparison = result.comparisons.get("accepted")?;
            (comparison.ratio > max_ratio).then(|| {
                format!(
                    "{} {} regressed by {:.2}%",
                    result.subject,
                    result.case_id,
                    (comparison.ratio - 1.0) * 100.0
                )
            })
        })
        .collect::<Vec<_>>();

    if regressions.is_empty() {
        Ok(AcceptanceReport {
            passed: true,
            message: String::new(),
        })
    } else {
        Ok(AcceptanceReport {
            passed: false,
            message: regressions.join("; "),
        })
    }
}

fn exact_match(result: &BenchResult, baseline: &BenchResult) -> bool {
    result.subject == baseline.subject
        && result.variant == baseline.variant
        && semantic_match(result, baseline)
}

fn semantic_match(result: &BenchResult, baseline: &BenchResult) -> bool {
    result.fixture_fingerprint == baseline.fixture_fingerprint
        && result.source_width == baseline.source_width
        && result.source_height == baseline.source_height
        && result.output_width == baseline.output_width
        && result.output_height == baseline.output_height
        && result.scale == baseline.scale
        && result.filter == baseline.filter
        && result.pixel_format == baseline.pixel_format
        && result.params_fingerprint == baseline.params_fingerprint
}

fn comparison(baseline: &str, current_ns: f64, baseline_ns: f64) -> ComparisonReport {
    let ratio = current_ns / baseline_ns;
    ComparisonReport {
        baseline: baseline.to_owned(),
        median_ns: baseline_ns,
        ratio,
        status: if ratio < 0.98 {
            "faster".to_owned()
        } else if ratio > 1.02 {
            "slower".to_owned()
        } else {
            "same".to_owned()
        },
    }
}
