//! Minimal CLI flag parsing and help text.

use std::collections::BTreeMap;

use crate::error::BenchError;

#[derive(Debug, Default)]
pub(crate) struct Flags {
    values: BTreeMap<String, String>,
}

impl Flags {
    pub(crate) fn parse(args: &[String]) -> Result<Self, BenchError> {
        let mut values = BTreeMap::new();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if let Some((key, value)) = arg.split_once('=') {
                values.insert(normalize_key(key).to_owned(), value.to_owned());
                index += 1;
                continue;
            }
            if arg.starts_with("--") {
                if args
                    .get(index + 1)
                    .is_none_or(|next| next.starts_with("--"))
                {
                    values.insert(normalize_key(arg).to_owned(), "true".to_owned());
                    index += 1;
                    continue;
                }

                let value = args
                    .get(index + 1)
                    .expect("checked next argument exists above");
                values.insert(normalize_key(arg).to_owned(), value.to_owned());
                index += 2;
                continue;
            }
            return Err(BenchError::Config(format!("unexpected argument {arg:?}")));
        }
        Ok(Self { values })
    }

    pub(crate) fn optional(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub(crate) fn present(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

fn normalize_key(key: &str) -> &str {
    match key {
        "--samples" | "--iterations" | "--measurement-iterations" => "--sample-size",
        "--warmup-iterations" => "--warm-up-iterations",
        "--warmup-time" => "--warm-up-time",
        "--target-sample-ms" => "--target-sample-time",
        "--inter-sample-delay-ms" => "--inter-sample-delay",
        _ => key,
    }
}

pub(crate) fn split_domain(args: &[String]) -> Result<(&str, &[String]), BenchError> {
    let domain = args
        .first()
        .ok_or_else(|| BenchError::Config("expected domain argument".to_owned()))?;
    Ok((domain, &args[1..]))
}

pub(crate) fn csv_set(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(crate) fn help_text() -> &'static str {
    "Usage:\n  ditherette-bench list-subjects [--domain resize] [--path FILE]\n  ditherette-bench describe-subject SUBJECT\n  ditherette-bench run PROFILE [overrides...]\n  ditherette-bench perf resize --subjects SUBJECTS [--profile PROFILE] [--fixtures FILE_OR_STEM,...] [--scales 0.5 | --scale-group upscale,downscale | --scale-pairs 0.5x1,1x0.5]\n  ditherette-bench comp resize --left SUBJECT_OR_PATH --right SUBJECT_OR_PATH\n  ditherette-bench tile resize --subject SUBJECT\n  ditherette-bench tiling-sweep [--filters nearest,area,...] [--fixtures Celeste_Insta_selfie,Celeste_box_art] [--band-heights 32,48,...,full] [--min-band-height 32] [--worker-counts 1,2,4,8]\n\nManifest flags:\n  --profile NAME              Load defaults plus [profiles.NAME] from crates/ditherette-bench/ditherette-bench.toml.\n  --config PATH               Use an alternate TOML manifest. CLI flags override manifest values.\n\nMeasurement flags for perf/comp:\n  --sample-size N             Stop timed measurement after N samples. Default: 100.\n  --samples N                 Alias for --sample-size.\n  --iterations N              Legacy alias for --sample-size.\n  --measurement-time DURATION Stop timed measurement after DURATION. Default: 5s.\n  --warm-up-time DURATION     Warm up until this duration is reached. Default: 1s.\n  --warm-up-iterations N      Optional warmup iteration target after calibration refinements.\n  --target-sample-time MS     Warmup calibration target per timed batch. Default: auto = measurement-time/sample-size.\n  --sample-mode MODE          throughput batches calibrated iterations; interactive measures one resize per sample.\n  --cache-state STATE         warm or scrubbed. Scrubbed touches a cache-scrub buffer before each sample.\n  --cache-scrub-size SIZE     Buffer size for --cache-state scrubbed. Default: 64MiB.\n  --inter-sample-delay D      Sleep outside timing before each measured sample. Default: 0ms.\n  --live-stats                Update the time/thrpt/prct/stat/range block during measurement.\n  --preheat-time DURATION     Spin before measurement to wake/boost CPUs. Default from manifest: 5s.\n  --process-priority MODE     normal or high. High requests best-effort OS priority/QoS/P-core hints.\n\nCorrectness/comparison flags for perf:\n  --oracle SUBJECT           Verify all subjects against this subject before timing; also compare timings to it.\n  --correctness MODE         exact (default) or bounded. Bounded compares RGBA Euclidean color distance per pixel.\n  --max-color-distance N     Maximum per-pixel RGBA Euclidean distance allowed with bounded correctness. Default: 2.0.\n  --max-mean-color-distance N  Maximum mean per-pixel RGBA Euclidean distance. Default: 2.0.\n  --max-rms-color-distance N Maximum RMS per-pixel RGBA Euclidean distance. Default: 2.0.\n  --allow-correctness-failures  Continue timing after oracle failures; prints a red manual-review warning.\n  --output-img DIR           Write PNG resize outputs under DIR for each oracle/perf subject, fixture, and scale.\n  --save-oracle SUBJECT      Force-refresh/replace the matching oracle baseline before the normal run.\n  --save-oracle              Force-refresh/replace the --oracle subject.\n  --replace-oracle SUBJECT   Alias for --save-oracle SUBJECT.\n  --replace-oracle           Alias for --save-oracle.\n\nBaseline flags for perf:\n  --baseline NAME             Compare against accepted baseline NAME.\n  --save-baseline NAME        Run, then save this run as the accepted baseline, overwriting if it exists.\n  --replace-baseline NAME     Do not run; replace accepted baseline NAME from latest compatible indexed case runs.\n  --no-run                    Deprecated no-op with --replace-baseline; invalid otherwise.\n\nDurations accept bare seconds for --measurement-time/--warm-up-time, bare milliseconds for *-ms flags, or explicit suffixes like 500ms and 5s.\n"
}
