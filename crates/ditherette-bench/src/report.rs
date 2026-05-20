//! Terminal report rendering.

use std::io::{self, IsTerminal, Write};
use std::time::{Duration, Instant};

use ditherette_bench_api::SubjectId;

use crate::{
    fixture::Fixture,
    measure::{MeasurementConfig, MeasurementObserver, MeasurementProgress},
    result::{BenchResult, ComparisonReport, SampleStats},
    runtime::RuntimeTuningReport,
    util::{dim, format_comparison, format_duration, format_ns, format_significant, heading},
};

pub(crate) fn log_perf_start(
    domain: &str,
    subjects: &[String],
    fixtures: &[Fixture],
    scales: &[f64],
    measurement: &MeasurementConfig,
    oracle: Option<&SubjectId>,
    baseline: Option<&str>,
) {
    println!("{}", heading("Ditherette perf benchmark"));
    println!("  domain:   {domain}");
    println!("  subjects: {}", subjects.join(", "));
    println!("  scales:   {}", format_scales(scales));
    println!(
        "  oracle:   {}",
        oracle
            .map(ToString::to_string)
            .unwrap_or_else(|| "—".to_owned())
    );
    println!("  baseline: {}", baseline.unwrap_or("—"));
    println!(
        "  config:   {}, warmup {}, target batch {}, run until {} samples or {}",
        measurement.sample_mode().as_str(),
        format_duration(measurement.warmup_time()),
        format_duration(measurement.target_sample()),
        measurement.sample_size(),
        format_duration(measurement.measurement_time())
    );
    if measurement.cache_state().as_str() != "warm" {
        println!(
            "            cache {}, scrub {}, inter-sample delay {}",
            measurement.cache_state().as_str(),
            format_bytes(measurement.cache_scrub_size()),
            format_duration(measurement.inter_sample_delay())
        );
    } else if !measurement.inter_sample_delay().is_zero() {
        println!(
            "            cache warm, inter-sample delay {}",
            format_duration(measurement.inter_sample_delay())
        );
    }
    if measurement.live_stats() {
        println!("            live stats enabled");
    }
    if !measurement.preheat_time().is_zero() || measurement.process_priority().as_str() != "normal"
    {
        println!(
            "            priority {}, preheat {}",
            measurement.process_priority().as_str(),
            format_duration(measurement.preheat_time())
        );
    }
    if let Some(iterations) = measurement.warmup_iterations() {
        println!("            warmup iteration cap: {iterations}");
    }
    println!("  fixtures:");
    for fixture in fixtures {
        println!(
            "    - {} [{}] {}x{}",
            fixture.id, fixture.kind, fixture.width, fixture.height
        );
    }
    println!();
}

pub(crate) fn log_runtime_tuning(report: &RuntimeTuningReport) {
    println!(
        "{} priority {}",
        heading("Runtime"),
        report.priority.as_str()
    );
    for note in &report.priority_notes {
        println!("  {note}");
    }
    for note in &report.affinity_notes {
        println!("  {note}");
    }
    println!();
}

pub(crate) fn log_preheat_start(duration: Duration) {
    if !duration.is_zero() {
        println!(
            "{} warming CPU for {}",
            heading("Preheat"),
            format_duration(duration)
        );
    }
}

pub(crate) fn log_correctness_start(total_checks: usize) {
    println!("{} {} checks", heading("Correctness"), total_checks);
}

pub(crate) fn log_correctness_ok(subject: &str, oracle: &SubjectId, case: &str) {
    println!("  ok {subject} vs {oracle} · {case}");
}

pub(crate) struct MeasurementLogger {
    measurement: MeasurementConfig,
    live: bool,
    live_stats: bool,
    warmup_open: bool,
    rendered_lines: usize,
    last_render: Option<Instant>,
}

impl MeasurementLogger {
    pub(crate) fn new(subject: &str, case: &str, measurement: MeasurementConfig) -> Self {
        println!("{} {subject} · {case}", heading("Benchmarking"));
        let live = io::stdout().is_terminal();
        if live {
            print!("  warmup: {}", format_duration(measurement.warmup_time()));
            let _ = io::stdout().flush();
        }
        Self {
            measurement,
            live,
            live_stats: measurement.live_stats(),
            warmup_open: live,
            rendered_lines: 0,
            last_render: None,
        }
    }

    pub(crate) fn finish(&mut self, result: &BenchResult) {
        if self.warmup_open {
            println!();
            self.warmup_open = false;
        }
        if self.live && self.rendered_lines > 0 {
            move_cursor_to_block_start(self.rendered_lines);
        }
        let lines = render_measurement_block(&MeasurementBlock::from_result(
            result,
            self.measurement.sample_size(),
            self.measurement.measurement_time(),
        ));
        print_lines(&lines, self.live);
        self.rendered_lines = lines.len();
        println!();
    }
}

impl MeasurementObserver for MeasurementLogger {
    fn warmup_batch(&mut self, batch_size: usize, elapsed: Duration) {
        let line = format!(
            "  warmup: {} (batch size: {}, taking {})",
            format_duration(self.measurement.warmup_time()),
            batch_size,
            format_duration(elapsed)
        );
        if self.live {
            print!("\r\x1b[2K{line}");
            let _ = io::stdout().flush();
        }
    }

    fn warmup_finished(&mut self, batch_size: usize, elapsed: Duration) {
        let line = format!(
            "  warmup: {} (batch size: {}, taking {})",
            format_duration(self.measurement.warmup_time()),
            batch_size,
            format_duration(elapsed)
        );
        if self.live {
            print!("\r\x1b[2K{line}\n");
            let _ = io::stdout().flush();
        } else {
            println!("{line}");
        }
        self.warmup_open = false;
    }

    fn measurement_progress(
        &mut self,
        progress: MeasurementProgress,
        samples: &[f64],
        output: (u32, u32),
    ) -> bool {
        if !self.live {
            return false;
        }
        let complete = progress.samples_done >= progress.sample_size
            || progress.elapsed >= progress.measurement_time;
        let now = Instant::now();
        if progress.samples_done > 0
            && !complete
            && self.last_render.is_some_and(|last_render| {
                now.duration_since(last_render) < Duration::from_millis(100)
            })
        {
            return false;
        }
        self.last_render = Some(now);
        if self.rendered_lines > 0 {
            move_cursor_to_block_start(self.rendered_lines);
        }
        if self.live_stats && !samples.is_empty() {
            let lines = render_measurement_block(&MeasurementBlock::from_samples(
                progress, output, samples,
            ));
            print_lines(&lines, self.live);
            self.rendered_lines = lines.len();
        } else {
            println!("\x1b[2K{}", render_measure_line(progress));
            self.rendered_lines = 1;
        }
        let _ = io::stdout().flush();
        true
    }
}

fn render_measure_line(progress: MeasurementProgress) -> String {
    format!(
        "  measure: {}/{} smp | {}/{} | {} iter",
        progress.samples_done,
        progress.sample_size,
        format_progress_duration(progress.elapsed),
        format_progress_duration(progress.measurement_time),
        progress.total_iterations
    )
}

struct MeasurementBlock {
    samples_done: usize,
    sample_size: usize,
    elapsed: Duration,
    measurement_time: Duration,
    total_iterations: usize,
    output: (u32, u32),
    stats: Option<SampleStats>,
    accepted_comparison: Option<ComparisonReport>,
}

impl MeasurementBlock {
    fn from_result(result: &BenchResult, sample_size: usize, measurement_time: Duration) -> Self {
        let elapsed_ns = result.sample_ns.iter().sum::<f64>() * result.iterations_per_sample as f64;
        Self {
            samples_done: result.samples,
            sample_size,
            elapsed: Duration::from_nanos(elapsed_ns.max(0.0) as u64),
            measurement_time,
            total_iterations: result.total_iterations,
            output: (result.output_width, result.output_height),
            stats: Some(SampleStats::from_samples(&result.sample_ns)),
            accepted_comparison: result.comparisons.get("accepted").cloned(),
        }
    }

    fn from_samples(progress: MeasurementProgress, output: (u32, u32), samples: &[f64]) -> Self {
        Self {
            samples_done: progress.samples_done,
            sample_size: progress.sample_size,
            elapsed: progress.elapsed,
            measurement_time: progress.measurement_time,
            total_iterations: progress.total_iterations,
            output,
            stats: Some(SampleStats::from_samples(samples)),
            accepted_comparison: None,
        }
    }
}

fn render_measurement_block(block: &MeasurementBlock) -> Vec<String> {
    let mut lines = vec![format!(
        "  measure: {}/{} smp | {}/{} | {} iter",
        block.samples_done,
        block.sample_size,
        format_progress_duration(block.elapsed),
        format_progress_duration(block.measurement_time),
        block.total_iterations
    )];

    let Some(stats) = block.stats else {
        lines.push("    time:   [— — —]".to_owned());
        lines.push("    thrpt:  [— — —]".to_owned());
        lines.push("  prct:    p50  p75  p90  p95  p99".to_owned());
        lines.push("          —    —    —    —    —".to_owned());
        lines.push("  stat:    mean  mode  stdev".to_owned());
        lines.push("          —     —     —".to_owned());
        lines.push("  range:   [— —]".to_owned());
        return lines;
    };

    let time_lower = (stats.mean_ns - stats.stdev_ns).max(0.0);
    let time_upper = stats.mean_ns + stats.stdev_ns;
    let throughput_lower = output_mpix_per_s(block.output, time_upper);
    let throughput_mean = output_mpix_per_s(block.output, stats.mean_ns);
    let throughput_upper = output_mpix_per_s(block.output, time_lower);

    lines.push(format!(
        "    time:   [{} {} {}]",
        dim(format_ns(time_lower)),
        format_ns(stats.mean_ns),
        dim(format_ns(time_upper))
    ));
    lines.push(format!(
        "    thrpt:  [{} {} {}]",
        dim(format_mpix_per_s(throughput_lower)),
        format_mpix_per_s(throughput_mean),
        dim(format_mpix_per_s(throughput_upper))
    ));

    if let Some(comparison) = block.accepted_comparison.as_ref() {
        lines.extend(render_change_block(&stats, comparison));
    }

    let percentiles = [
        ("p50", format_ns(stats.median_ns)),
        ("p75", format_ns(stats.p75_ns)),
        ("p90", format_ns(stats.p90_ns)),
        ("p95", format_ns(stats.p95_ns)),
        ("p99", format_ns(stats.p99_ns)),
    ];
    let percentile_widths = cell_widths(&percentiles);
    lines.push(format!(
        "  prct:   {}",
        format_cells(
            percentiles.iter().map(|(label, _)| *label),
            &percentile_widths
        )
    ));
    lines.push(format!(
        "          {}",
        format_cells(
            percentiles.iter().map(|(_, value)| value.as_str()),
            &percentile_widths
        )
    ));

    let stats_cells = [
        ("mean", format_ns(stats.mean_ns)),
        ("mode", format_ns(stats.mode_ns)),
        ("stdev", format_ns(stats.stdev_ns)),
    ];
    let stats_widths = cell_widths(&stats_cells);
    lines.push(format!(
        "  stat:   {}",
        format_cells(stats_cells.iter().map(|(label, _)| *label), &stats_widths)
    ));
    lines.push(format!(
        "          {}",
        format_cells(
            stats_cells.iter().map(|(_, value)| value.as_str()),
            &stats_widths
        )
    ));
    lines.push(format!(
        "  range:   [{} {}]",
        dim(format_ns(stats.min_ns)),
        dim(format_ns(stats.max_ns))
    ));
    lines
}

fn render_change_block(stats: &SampleStats, comparison: &ComparisonReport) -> Vec<String> {
    let time_lower = percent_change(stats.median_ns - stats.stdev_ns, comparison.median_ns);
    let time_median = percent_change(stats.median_ns, comparison.median_ns);
    let time_upper = percent_change(stats.median_ns + stats.stdev_ns, comparison.median_ns);
    let throughput_lower =
        speed_change_percent(stats.median_ns + stats.stdev_ns, comparison.median_ns);
    let throughput_median = speed_change_percent(stats.median_ns, comparison.median_ns);
    let throughput_upper =
        speed_change_percent(stats.median_ns - stats.stdev_ns, comparison.median_ns);
    let verdict = match comparison.status.as_str() {
        "faster" => "Performance has improved.",
        "slower" => "Performance has regressed.",
        _ => "Change within noise threshold.",
    };

    vec![
        "  change:".to_owned(),
        format!(
            "    time:   [{} {} {}]",
            format_delta_percent(time_lower),
            format_delta_percent(time_median),
            format_delta_percent(time_upper)
        ),
        format!(
            "    thrpt:  [{} {} {}]",
            format_delta_percent(throughput_lower),
            format_delta_percent(throughput_median),
            format_delta_percent(throughput_upper)
        ),
        format!("    {verdict}"),
    ]
}

fn percent_change(current: f64, baseline: f64) -> f64 {
    (current / baseline - 1.0) * 100.0
}

fn speed_change_percent(current: f64, baseline: f64) -> f64 {
    (baseline / current - 1.0) * 100.0
}

fn format_delta_percent(value: f64) -> String {
    let sign = if value < 0.0 { "−" } else { "+" };
    format!("{sign}{:.4}%", value.abs())
}

pub(crate) fn print_perf_table(results: &[BenchResult]) {
    println!("{}", heading("Summary"));
    let rows = results
        .iter()
        .map(|result| {
            vec![
                result.subject.clone(),
                result.case_id.clone(),
                format_ns(result.mean_ns),
                format_ns(result.median_ns),
                format_ns(result.stdev_ns),
                format_ns(result.p95_ns),
                format_significant(result.output_mpix_per_s, 5),
                format_comparison(result.comparisons.get("accepted")),
                format_comparison(result.comparisons.get("oracle")),
                format_comparison(result.comparisons.get("spec")),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &[
            "subject", "case", "mean", "median", "stdev", "p95", "MPix/s", "baseline", "oracle",
            "spec",
        ],
        &rows,
        &[
            false, false, true, true, true, true, true, false, false, false,
        ],
    );
}

pub(crate) fn print_comp_table(results: &[BenchResult], left: &SubjectId, right: &SubjectId) {
    println!("{}", heading("Summary"));
    let rows = results
        .iter()
        .filter(|result| result.subject == left.as_str() || result.subject == right.as_str())
        .map(|result| {
            vec![
                result.case_id.clone(),
                result.subject.clone(),
                format_ns(result.median_ns),
                format_ns(result.p95_ns),
                format_comparison(result.comparisons.get("left")),
                result.verified.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    print_table(
        &["case", "subject", "median", "p95", "ratio", "verified"],
        &rows,
        &[false, false, true, true, false, false],
    );
}

fn cell_widths(cells: &[(&str, String)]) -> Vec<usize> {
    cells
        .iter()
        .map(|(label, value)| label.len().max(visible_width(value)))
        .collect()
}

fn format_cells<'a>(values: impl Iterator<Item = &'a str>, widths: &[usize]) -> String {
    values
        .zip(widths)
        .map(|(value, width)| format!("{:>width$}", value, width = width))
        .collect::<Vec<_>>()
        .join("  ")
}

fn print_lines(lines: &[String], clear: bool) {
    for line in lines {
        if clear {
            println!("\x1b[2K{line}");
        } else {
            println!("{line}");
        }
    }
}

fn move_cursor_to_block_start(lines: usize) {
    print!("\x1b[{lines}A");
}

fn format_progress_duration(duration: Duration) -> String {
    format!("{:.2}s", duration.as_secs_f64())
}

fn print_table(headers: &[&str], rows: &[Vec<String>], right_align: &[bool]) {
    let widths = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            rows.iter()
                .filter_map(|row| row.get(index))
                .map(|value| visible_width(value))
                .max()
                .unwrap_or(0)
                .max(header.len())
        })
        .collect::<Vec<_>>();

    for (index, header) in headers.iter().enumerate() {
        if index > 0 {
            print!("  ");
        }
        print_cell(
            header,
            widths[index],
            right_align[index],
            index + 1 < headers.len(),
        );
    }
    println!();

    for row in rows {
        for (index, value) in row.iter().enumerate() {
            if index > 0 {
                print!("  ");
            }
            print_cell(
                value,
                widths[index],
                right_align[index],
                index + 1 < row.len(),
            );
        }
        println!();
    }
}

fn print_cell(value: &str, width: usize, right_align: bool, pad_right: bool) {
    let padding = width.saturating_sub(visible_width(value));
    if right_align {
        print!("{}{}", " ".repeat(padding), value);
    } else if pad_right {
        print!("{}{}", value, " ".repeat(padding));
    } else {
        print!("{value}");
    }
}

fn visible_width(value: &str) -> usize {
    let mut width = 0;
    let mut chars = value.chars();
    while let Some(char) = chars.next() {
        if char == '\x1b' {
            for escaped in chars.by_ref() {
                if escaped == 'm' {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }
    width
}

fn output_mpix_per_s(output: (u32, u32), ns: f64) -> f64 {
    if ns <= 0.0 {
        return 0.0;
    }
    let output_pixels = f64::from(output.0) * f64::from(output.1);
    output_pixels / (ns / 1_000_000_000.0) / 1_000_000.0
}

fn format_mpix_per_s(value: f64) -> String {
    format!("{} MPix/s", format_significant(value, 5))
}

fn format_bytes(bytes: usize) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const KIB: f64 = 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{} MiB", format_significant(bytes as f64 / MIB, 5))
    } else if bytes >= 1024 {
        format!("{} KiB", format_significant(bytes as f64 / KIB, 5))
    } else {
        format!("{bytes} B")
    }
}

fn format_scales(scales: &[f64]) -> String {
    scales
        .iter()
        .map(|scale| scale.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
