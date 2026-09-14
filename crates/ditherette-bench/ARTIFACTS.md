# Ditherette bench artifacts

Read [exclusive execution](EXECUTION.md) before collecting timings. Prepare builds,
drain implementation work, then run prebuilt executables under the shared lease.

[Three-way verification](VERIFICATION.md) defines typed result identities,
exactness requirements, and retained raw/PNG review bundles.

[Fresh paired performance](PAIRED.md) owns the regression gate. The legacy
historical comparisons described below are diagnostic, not fresh release evidence.

`ditherette-bench` writes benchmark run and baseline artifacts as JSON under:

```text
crates/ditherette-bench/target/bench/
  latest/<command>-<domain>.json
  baselines/<role>/<name>.json                         # legacy whole-run/spec baselines
  baselines/<command>/<domain>/<bench-key>/<config-key>/<fixture>/<scale>/<name>/<run-id>.json
  <command>/<domain>/<bench-key>/<config-key>/[baseline-name/]<fixture>/<scale>/run.json
```

Artifacts under the crate-local `target/` are intentionally Criterion-like: generated benchmark state belongs in build output, not source-controlled project data, unless copied elsewhere with `--out`.

Per-case run files are deterministic and overwritten in place, not timestamped. Scoped baseline files are per-case and timestamped by `run-id` under the baseline name. `bench-key` is derived from the measured subject. `config-key` is derived from command/domain/profile, measurement settings, subject id, filter, variant, pixel format, and params. Fixture fingerprint and scale are path scopes below the config, not config hash inputs. The optional baseline-name directory in run output is present when `--save-baseline <key>` is used.

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

Accepted baselines are scoped per case while sharing a human-readable name. The same name, such as `accepted`, may coexist for different profiles, subjects, fixtures, scales, and measurement configs because the path scopes are explicit:

```text
baselines/perf/resize/prod-resize-area-scalar/<config>/Celeste_Insta_selfie/0-5x/accepted/<run-id>.json
```

Subset runs reuse matching per-fixture/per-scale baseline files without reestablishing the whole matrix. Uniform scales keep the legacy `0-5x` key; anisotropic scales use separate `0-5x-1y`-style keys so width-only and height-only cases cannot collide with uniform cases.

Accepted baseline comparisons still require an exact matching entry:

- command
- domain
- measurement config
- subject id
- filter
- variant
- fixture fingerprint
- source dimensions
- output dimensions
- scale_x / scale_y
- pixel format
- params fingerprint

When a requested accepted baseline name has missing per-case entries, the run attaches comparisons for cases already present and leaves missing cases blank (`—`) in the baseline column. Dirty git trees are allowed and recorded.

If no accepted baseline is specified, `perf` compares exact-subject results to the latest compatible indexed per-case runs, then overwrites those indexed runs with the current results. `--save-baseline NAME` runs the benchmark and overwrites accepted baseline `NAME` for every measured case. `--replace-baseline NAME` does not run; it rebuilds the requested profile shape and overwrites accepted baseline `NAME` from the latest compatible indexed per-case runs.

Oracle baselines use the same per-case scoped layout under the baseline name `oracle`. When `--oracle SUBJECT` is provided, the harness loads matching oracle cases and measures only missing requested cases before the normal run, so partial oracle baselines fill themselves incrementally. `--save-oracle SUBJECT` refreshes/replaces the requested oracle cases before the normal run; bare `--save-oracle` refreshes the `--oracle` subject. `--replace-oracle` is an alias for `--save-oracle`.

## Oracle/spec matching

Oracle and spec comparisons are intentionally looser than accepted baselines. They match semantic resize cases, not exact subject ids:

- fixture fingerprint
- source dimensions
- output dimensions
- scale_x / scale_y
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
  "scale_x": 0.95,
  "scale_y": 0.95,
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

## Nearest old-Criterion comparison snapshot

2026-05-14 snapshot saved to avoid rerunning the long old-crate Criterion parity bench just to recover numbers.

Old Criterion command:

```sh
cargo bench --manifest-path crates/ditherette-wasm-old/Cargo.toml --bench crit_resize_nearest
```

The old bench was patched to match `ditherette-bench.toml` nearest defaults: fixtures `Celeste_Insta_selfie.png` and `Celeste_box_art.png`; scales `0.1, 0.125, 0.25, 0.5, 0.75, 0.9, 0.95, 0.99, 1.01, 1.05, 1.25, 1.5, 2, 4`; sample size `100`; measurement time `5s`; warmup `1s`; output dimensions rounded like the custom harness.

Current numbers are from accepted custom-bench baseline. `cur vs old` is speed-relative from mean times: positive means current prod is faster than old.

| fixture | scale | current mean | old mean | cur vs old |
|---|---:|---:|---:|---:|
| Celeste_Insta_selfie | 0.1x | 2.251 µs | 2.176 µs | -3.3% |
| Celeste_Insta_selfie | 0.125x | 2.947 µs | 2.758 µs | -6.4% |
| Celeste_Insta_selfie | 0.25x | 9.228 µs | 8.195 µs | -11.2% |
| Celeste_Insta_selfie | 0.5x | 31.325 µs | 26.437 µs | -15.6% |
| Celeste_Insta_selfie | 0.75x | 97.676 µs | 59.434 µs | -39.2% |
| Celeste_Insta_selfie | 0.9x | 140.618 µs | 180.098 µs | +28.1% |
| Celeste_Insta_selfie | 0.95x | 79.977 µs | 73.276 µs | -8.4% |
| Celeste_Insta_selfie | 0.99x | 65.532 µs | 55.505 µs | -15.3% |
| Celeste_Insta_selfie | 1.01x | 174.812 µs | 104.659 µs | -40.1% |
| Celeste_Insta_selfie | 1.05x | 189.411 µs | 111.957 µs | -40.9% |
| Celeste_Insta_selfie | 1.25x | 267.959 µs | 157.033 µs | -41.4% |
| Celeste_Insta_selfie | 1.5x | 382.192 µs | 228.022 µs | -40.3% |
| Celeste_Insta_selfie | 2x | 671.828 µs | 401.666 µs | -40.2% |
| Celeste_Insta_selfie | 4x | 2735.824 µs | 1566.540 µs | -42.7% |
| Celeste_box_art | 0.1x | 42.363 µs | 39.542 µs | -6.7% |
| Celeste_box_art | 0.125x | 52.478 µs | 50.247 µs | -4.3% |
| Celeste_box_art | 0.25x | 174.062 µs | 156.456 µs | -10.1% |
| Celeste_box_art | 0.5x | 720.805 µs | 667.063 µs | -7.5% |
| Celeste_box_art | 0.75x | 1744.770 µs | 1133.879 µs | -35.0% |
| Celeste_box_art | 0.9x | 2415.533 µs | 3217.213 µs | +33.2% |
| Celeste_box_art | 0.95x | 1590.431 µs | 1546.192 µs | -2.8% |
| Celeste_box_art | 0.99x | 1515.477 µs | 1476.660 µs | -2.6% |
| Celeste_box_art | 1.01x | 3066.768 µs | 1943.027 µs | -36.6% |
| Celeste_box_art | 1.05x | 3275.258 µs | 2064.467 µs | -37.0% |
| Celeste_box_art | 1.25x | 4569.409 µs | 2934.479 µs | -35.8% |
| Celeste_box_art | 1.5x | 6606.771 µs | 4083.916 µs | -38.2% |
| Celeste_box_art | 2x | 11557.093 µs | 7139.055 µs | -38.2% |
| Celeste_box_art | 4x | 46853.405 µs | 26999.089 µs | -42.4% |

Takeaways: current prod is faster than old at `0.9x`; near parity on large `0.95x`/`0.99x`; old remains substantially faster for upscales and `0.75x`.
