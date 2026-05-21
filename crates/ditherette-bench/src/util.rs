//! Small shared helpers for paths, formatting, and checksums.

use std::time::Duration;

use ditherette_bench_api::SubjectId;

use crate::result::ComparisonReport;

pub(crate) const RGBA_CHANNELS: usize = 4;

pub(crate) fn checksum(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn output_dimensions(
    source_width: u32,
    source_height: u32,
    scale_x: f64,
    scale_y: f64,
) -> (u32, u32) {
    (
        ((f64::from(source_width) * scale_x).round() as u32).max(1),
        ((f64::from(source_height) * scale_y).round() as u32).max(1),
    )
}

pub(crate) fn normalize_path_string(path: impl AsRef<str>) -> String {
    path.as_ref().replace('\\', "/")
}

pub(crate) fn optional_id(id: &Option<SubjectId>) -> String {
    id.as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "—".to_owned())
}

pub(crate) fn format_comparison(comparison: Option<&ComparisonReport>) -> String {
    comparison
        .map(|comparison| {
            let speed_change_percent = ((1.0 / comparison.ratio) - 1.0) * 100.0;
            let text = format!("{speed_change_percent:+.2}%");
            match comparison.status.as_str() {
                "faster" => green(&text),
                "slower" => red(&text),
                _ => text,
            }
        })
        .unwrap_or_else(|| "—".to_owned())
}

pub(crate) fn heading(text: impl AsRef<str>) -> String {
    color("1;36", text.as_ref())
}

pub(crate) fn dim(text: impl AsRef<str>) -> String {
    color("2", text.as_ref())
}

pub(crate) fn green(text: impl AsRef<str>) -> String {
    color("32", text.as_ref())
}

pub(crate) fn red(text: impl AsRef<str>) -> String {
    color("31", text.as_ref())
}

pub(crate) fn bold_red(text: impl AsRef<str>) -> String {
    color("1;31", text.as_ref())
}

pub(crate) fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs_f64();
    if seconds >= 1.0 {
        format!("{seconds:.2}s")
    } else {
        format!("{:.0}ms", seconds * 1_000.0)
    }
}

fn color(code: &str, text: &str) -> String {
    format!("\x1b[{code}m{text}\x1b[0m")
}

pub(crate) fn format_ns(ns: f64) -> String {
    let (value, unit) = if ns >= 1_000_000_000.0 {
        (ns / 1_000_000_000.0, "s")
    } else if ns >= 1_000_000.0 {
        (ns / 1_000_000.0, "ms")
    } else if ns >= 1_000.0 {
        (ns / 1_000.0, "µs")
    } else {
        (ns, "ns")
    };
    format!("{} {unit}", format_significant(value, 5))
}

pub(crate) fn format_significant(value: f64, significant_digits: usize) -> String {
    if !value.is_finite() || value == 0.0 {
        return format!("{value:.1}");
    }

    let magnitude = value.abs().log10().floor() as isize + 1;
    let precision = (significant_digits as isize - magnitude).max(0) as usize;
    format!("{value:.precision$}")
}
