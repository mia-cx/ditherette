# Ditherette bench artifacts

`ditherette-bench` writes benchmark run and baseline artifacts as JSON under:

```text
crates/ditherette-bench/target/bench/
  latest/<command>-<domain>.json
  baselines/<role>/<name>.json
  <command>/<domain>/<bench-key>/<config-key>/[baseline-name/]<fixture>/<scale>/run.json
```

Artifacts under the crate-local `target/` are intentionally Criterion-like: generated benchmark state belongs in build output, not source-controlled project data, unless copied elsewhere with `--out`.

Per-case run files are deterministic and overwritten in place, not timestamped. `bench-key` is derived from the measured subject. `config-key` is derived from measurement and subject configuration. The optional baseline-name directory is present when `--save-baseline <key>` is used.

## Envelope

All saved artifacts use the same envelope:

```json
{
  "schema": "ditherette-bench-artifact",
  "schema_version": 1,
  "artifact_kind": "run",
  "run_id": "1779259606-312439000",
  "created_at_unix": 1779259606,
  "tool": { "name": "ditherette-bench", "version": "0.1.0" },
  "command": "perf",
  "domain": "resize",
  "cli": ["ditherette-bench", "perf", "resize"],
  "git": { "commit": "...", "branch": "...", "dirty": true },
  "host": { "os": "macos", "arch": "aarch64", "logical_cpus": 10 },
  "profile": "release",
  "measurement": {
    "sample_size": 500,
    "measurement_time_ms": 5000,
    "warmup_iterations": null,
    "warmup_time_ms": 1000,
    "target_sample_time_ms": 10,
    "sample_mode": "throughput",
    "cache_state": "warm",
    "cache_scrub_size": 67108864,
    "inter_sample_delay_ms": 0,
    "live_stats": false,
    "preheat_time_ms": 5000,
    "process_priority": "high"
  },
  "results": []
}
```

Loaders reject unknown schemas, future schema versions, and wrong artifact kinds.

## Baselines

Baseline artifacts use the same envelope with `artifact_kind = "baseline"` and baseline metadata:

```json
{
  "artifact_kind": "baseline",
  "baseline": {
    "name": "nearest-accepted",
    "role": "accepted",
    "source_run_id": "1779259606-312439000"
  }
}
```

Roles:

```text
accepted    approved performance state for exact-subject comparisons
oracle      auto-managed oracle characterization for the current config
spec        saved spec characterization
experiment  temporary comparison point
```

Accepted baselines must match the current run config exactly:

- command
- domain
- measurement config
- subject id
- filter
- variant
- fixture fingerprint
- source dimensions
- output dimensions
- scale
- pixel format
- params fingerprint

A missing or incompatible accepted baseline entry is an error. Dirty git trees are allowed and recorded.

If no accepted baseline is specified, `perf` compares exact-subject results to the most recent compatible previous run from `crates/ditherette-bench/target/bench/latest/perf-resize.json` before overwriting it with the current run.

Oracle baselines are auto-managed. When `--oracle SUBJECT` is provided, the harness loads the matching `oracle` baseline for the current command/domain/measurement/fixture/scale matrix. If it is missing or more than 24 hours old, the harness measures the oracle first, saves a fresh baseline, then runs the requested subjects. `--save-oracle SUBJECT` forces a refresh of that matching oracle baseline before the normal run; bare `--save-oracle` refreshes the `--oracle` subject.

## Oracle/spec matching

Oracle and spec comparisons are intentionally looser than accepted baselines. They match semantic resize cases, not exact subject ids:

- fixture fingerprint
- source dimensions
- output dimensions
- scale
- filter
- pixel format
- params fingerprint

This lets `prod:resize:nearest:tiled` compare against `spec:resize:nearest:scalar` for the same fixture/scale while preventing cross-fixture or cross-scale comparisons. Future subject descriptors can add richer semantic keys if filters gain equivalent variants with different names.

## Result rows

Each result records identity, correctness, raw samples, summary statistics, throughput, and attached comparisons:

```json
{
  "subject": "spec:resize:nearest:scalar",
  "case_id": "the_quilt_banner-1538x922-0.95x",
  "fixture": "the_quilt_banner",
  "fixture_kind": "file",
  "fixture_fingerprint": "...",
  "filter": "nearest",
  "variant": "scalar",
  "source_width": 1619,
  "source_height": 971,
  "output_width": 1538,
  "output_height": 922,
  "scale": 0.95,
  "pixel_format": "rgba8",
  "params_fingerprint": "resize-default",
  "verified": false,
  "verification": null,
  "checksum": "...",
  "samples": 3,
  "sample_ns": [17310.0, 17330.0, 17380.0],
  "iterations_per_sample": 577,
  "total_iterations": 1731,
  "min_ns": 17100.0,
  "median_ns": 17330.0,
  "mean_ns": 17340.0,
  "stdev_ns": 120.0,
  "mode_ns": 17330.0,
  "p75_ns": 17400.0,
  "p90_ns": 17500.0,
  "p95_ns": 17600.0,
  "p99_ns": 18000.0,
  "max_ns": 19000.0,
  "output_mpix_per_s": 623.9,
  "comparisons": {
    "accepted": {
      "baseline": "accepted",
      "median_ns": 17000.0,
      "ratio": 1.019,
      "status": "same"
    }
  }
}
```

`sample_ns` contains per-sample nanoseconds per operation. Each sample is one calibrated batch; `iterations_per_sample` records the batch size.

Throughput is currently output megapixels per second. GiB/s is intentionally omitted until subjects expose a credible bytes-processed contract.
