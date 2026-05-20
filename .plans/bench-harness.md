# Ditherette custom benchmark harness spec

## Status

Draft implementation contract. This document specifies the new custom benchmark harness design. It intentionally does **not** modify, replace, or depend on the existing Criterion, Brunch, quick-compare, tiling-sweep, or Wasm benchmark scripts.

## Goals

Build a Ditherette-specific image-processing benchmark harness that can:

- benchmark image-processing domains across multiple fixtures, scales, formats, and implementation variants
- compare current measurements against named accepted baselines
- compare current measurements against spec/oracle baselines in the same `perf` flow
- replace accepted baselines with latest accepted measurements to speed up perf loops
- sweep tiling policy parameters as a first-class benchmark mode
- compare two registered subjects or implementation files/modules as a first-class benchmark mode
- keep benchmark orchestration generic across domains without creating drift between `ditherette-wasm` and `ditherette-bench`
- preserve spec implementations as correctness/performance oracles without making `ditherette-bench` deep-import implementation internals

## Non-goals

- Do not rework existing benchmark scripts as part of the first implementation.
- Do not wrap Criterion or Brunch.
- Do not require production implementations before the harness can characterize spec implementations.
- Do not make source file paths part of stable benchmark identity.
- Do not use browser/Wasm benchmarks as the first native perf-loop tool.
- Do not build plots or HTML reports in the first version.
- Do not implement statistical significance testing in the first version.

## Repository layout

Target eventual layout:

```text
crates/
  ditherette-bench-api/
    Cargo.toml
    src/
      lib.rs
      id.rs
      descriptor.rs
      domain.rs
      resize.rs
      color.rs
      result.rs

  ditherette-bench/
    Cargo.toml
    src/
      main.rs
      cli.rs
      plan.rs
      registry.rs
      fixture.rs
      synthetic.rs
      checksum.rs
      verify.rs
      measure.rs
      baseline.rs
      report.rs
      commands/
        mod.rs
        perf.rs
        tile.rs
        comp.rs
      domains/
        mod.rs
        resize.rs
        color.rs

  ditherette-wasm/
    src/
      bench_subjects.rs              # compiled only behind bench-subjects feature
      spec/...                       # owns spec implementations and local adapters
      prod/...                       # future production implementations and local adapters

bench/
  plans/
    resize.toml
    color.toml
  results/
    runs/
    baselines/
      accepted/
      spec/
      experiment/
  README.md
```

The exact file split may change, but these package boundaries are part of the contract:

- `ditherette-bench-api` owns stable typed benchmark contracts.
- `ditherette-wasm` owns implementation adapters and subject registration.
- `ditherette-bench` owns CLI, plan expansion, measurement loops, baselines, and reports.

## Subject identity

All benchmarkable implementations use this stable subject ID format:

```text
module:domain:filter:variant
```

Field meanings:

```text
module   implementation namespace: spec, prod, old, external, experiment
          Examples: spec, prod, old, external

domain   image-processing domain
          Examples: resize, color, palette, quantize, dither

filter   algorithm, conversion, or operation family inside the domain
          Examples: nearest, area, bilinear, bicubic, lanczos3, oklab

variant  implementation or semantic policy variant
          Examples: scalar, tiled, simd, catmull-rom, scale-aware, mip-area
```

Examples:

```text
spec:resize:nearest:scalar
spec:resize:area:scalar
spec:resize:bilinear:scalar
spec:resize:bicubic:catmull-rom
spec:resize:lanczos3:scale-aware
spec:resize:trilinear:mip-area

prod:resize:nearest:scalar
prod:resize:nearest:tiled
prod:resize:area:scalar
prod:resize:area:tiled

spec:color:oklab:scalar
prod:color:oklab:tiled
prod:color:oklab:simd

prod:palette:median-cut:scalar
prod:dither:error-diffusion:tiled

old:resize:bilinear:scalar
external:resize:lanczos3:fast-image-resize
```

### Subject ID rules

- A subject ID has exactly four non-empty colon-separated segments.
- Subject IDs are semantic and stable across file moves.
- Source file paths are metadata only.
- Baselines match by subject ID and case semantics, not path.
- `comp` defaults require left and right subjects to share `domain` and `filter` unless explicitly overridden.
- Default oracle for a subject is:

```text
spec:<domain>:<filter>:scalar
```

unless the registry declares a different default oracle.

## Avoiding drift between crates

`ditherette-bench` must not deep-import internal implementation modules such as:

```rust
// forbidden in ditherette-bench
use ditherette_wasm::spec::resize::scalar::area::resize_area_into;
```

Instead, `ditherette-bench` imports only a registry exposed by `ditherette-wasm` behind a feature:

```rust
// allowed in ditherette-bench
let registry = ditherette_wasm::bench_subjects();
```

Implementation adapters live near the implementation they wrap. If an implementation signature changes, its local adapter fails to compile in `ditherette-wasm`; the bench harness does not silently drift.

### Required structure

```text
ditherette-bench-api:
  stable traits, input/output types, descriptors, subject IDs

ditherette-wasm:
  implementation-local adapters and subject registration

ditherette-bench:
  registry consumption, command orchestration, fixtures, timing, reporting
```

### Subject registration contract

Each subject descriptor records:

```rust
SubjectDescriptor {
    id: SubjectId,
    display_name: String,
    source_file: String,
    source_line: u32,
    default_oracle: Option<SubjectId>,
    capabilities: SubjectCapabilities,
    params_schema: ParamSchema,
}
```

`source_file` should normally come from `file!()` and `source_line` from `line!()` in the adapter/registration site.

File path lookup for `comp --left path --right path` must consult registry metadata. It must not infer subject IDs using hardcoded path rules except as optional suggestions.

If a file maps to zero subjects:

```text
error: no registered benchmark subject for path <path>
hint: run ditherette-bench list-subjects --path <path>
```

If a file maps to multiple subjects:

```text
error: path maps to multiple subjects
candidates:
  spec:resize:lanczos2:scale-aware
  spec:resize:lanczos3:scale-aware
hint: pass a subject ID explicitly
```

## Bench API crate contract

`ditherette-bench-api` provides shared types. It must remain small and stable.

### Generic IDs

```rust
pub struct SubjectId(String);      // module:domain:filter:variant

impl SubjectId {
    pub fn module(&self) -> &str;
    pub fn domain(&self) -> &str;
    pub fn filter(&self) -> &str;
    pub fn variant(&self) -> &str;
}

// Separate ModuleId/DomainId/FilterId/VariantId wrappers are intentionally deferred until they buy type-safety that outweighs ceremony.
```

Parsing validates syntax only. Registry validation decides whether a parsed ID exists.

### Generic descriptor

```rust
pub enum BenchSubject {
    Resize(ResizeBenchSubject),
}

pub struct ResizeBenchSubject {
    pub descriptor: SubjectDescriptor,
    pub resize_u8_rgba: ResizeU8RgbaFn,
}

pub type ResizeU8RgbaFn = for<'a> fn(
    ResizeInputU8Rgba<'a>,
    ResizeOutputU8Rgba<'a>,
    &ResizeParams,
) -> Result<(), BenchSubjectError>;
```

Function pointers are the current canonical subject adapter shape. They are small, cloneable, explicit, and avoid trait-object/lifetime ceremony. Future domains add enum variants with the same descriptor + typed function-pointer shape, e.g. `Color(ColorBenchSubject)`.

### Domain-specific typed contracts

Do not use an untyped `serde_json::Value -> Vec<u8>` subject interface for core benchmark execution. Use typed domain contracts.

The harness core is generic around:

- fixture loading
- case expansion
- warmup
- sample collection
- checksum
- verification
- baseline comparison
- report generation

Domain adapters are typed around:

- input/output image shape
- parameters
- pixel formats
- semantic verification defaults

## Resize domain contract

Initial typed resize contract:

```rust
pub type ResizeU8RgbaFn = for<'a> fn(
    ResizeInputU8Rgba<'a>,
    ResizeOutputU8Rgba<'a>,
    &ResizeParams,
) -> Result<(), BenchSubjectError>;

pub struct ResizeBenchSubject {
    pub descriptor: SubjectDescriptor,
    pub resize_u8_rgba: ResizeU8RgbaFn,
}
```

Trait-object subject contracts are deferred until a subject needs owned state or dynamic behavior that a function pointer cannot express.

Initial resize input/output:

```rust
pub struct ResizeInputU8Rgba<'a> {
    pub data: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub row_stride_elements: usize,
}

pub struct ResizeOutputU8Rgba<'a> {
    pub data: &'a mut [u8],
    pub width: u32,
    pub height: u32,
    pub row_stride_elements: usize,
}
```

Initial resize params:

```rust
pub struct ResizeParams {
    pub anchor: ResizeAnchorParam,
    pub support_policy: Option<SupportPolicyParam>,
    pub tile_policy: Option<TilePolicyParam>,
    pub extra: ParamMap,
}
```

The first version may only support packed RGBA8. The contract should leave room for later formats:

```text
Rgba8
Rgb8
LinearRgb32
LinearRgba32
Oklab32
Oklaba32
PaletteIndex8
```

### Color domain contract

Color operations should follow the same pattern:

```rust
pub trait ColorSubject {
    fn descriptor(&self) -> &SubjectDescriptor;

    fn transform(
        &self,
        input: ColorInput<'_>,
        output: ColorOutput<'_>,
        params: &ColorParams,
    ) -> Result<(), BenchSubjectError>;
}
```

Example subject IDs:

```text
spec:color:oklab:scalar
prod:color:oklab:tiled
prod:color:oklab:simd
```

## Commands

The first implementation must expose three specialized commands:

```text
ditherette-bench perf
ditherette-bench tile
ditherette-bench comp
```

All commands may accept a domain argument:

```sh
ditherette-bench perf resize ...
ditherette-bench tile resize ...
ditherette-bench comp resize ...
```

Future domain examples:

```sh
ditherette-bench perf color ...
ditherette-bench comp color ...
```

## `perf` command

### Purpose

Performance sweep across fixtures, scales, formats, filters, and subjects.

`perf` answers:

- how fast is this subject now?
- did this subject regress vs the accepted baseline?
- how does this subject compare to the spec/oracle baseline?

### Required CLI shape

```sh
ditherette-bench perf resize \
  --subjects prod:resize:area:tiled \
  --fixtures celeste,selfie,gradient \
  --scales 2,1,0.95,0.75,0.5,0.25,0.125 \
  --iterations 500 \
  --measurement-time 5s \
  --warm-up-time 1s \
  --out benchmark-results/runs/resize-area.json
```

### Subject selection

```sh
--subjects spec:resize:area:scalar,prod:resize:area:tiled
--module prod
--domain resize
--filters area,bilinear
--variants scalar,tiled
```

`--subjects` is exact. The filter/module/variant flags expand through the registry.

### Scale groups

Built-in scale groups:

```text
preview:
  0.5, 0.25

quick:
  2.0, 0.95, 0.75, 0.5, 0.25, 0.125

resize-critical:
  alias for quick

near-identity:
  0.95, 0.97, 0.98, 0.99, 1.0, 1.01, 1.02, 1.03, 1.05

  This group intentionally stays inside 0.95x..1.05x because larger deviations do not exercise the Apple Silicon near-identity slowdown we care about.

upscale:
  1.01, 1.02, 1.03, 1.05, 1.07, 1.1, 1.15, 1.25, 1.5, 1.67, 2.0, 2.38, 2.83, 3.36, 4.0, 4.76, 5.66, 6.73, 8.0

  This combines dense near-identity upscale probes with rounded logarithmic-ish spacing at larger scales. Identity itself is intentionally left to `near-identity` or explicit scale lists. Every value uses at most two decimal places.

downscale:
  0.99, 0.98, 0.97, 0.95, 0.93, 0.9, 0.85, 0.75, 0.5, 0.33, 0.25, 0.19, 0.16, 0.13

  This is an S-curve-style set: denser near identity, sparse around the middle/0.5x region, and denser again at very low scales, with at most two decimal places.

stress:
  4.0, 2.0, 0.5, 0.125

boundary:
  explicit tiny cases, not scale-derived only
```

CLI:

```sh
--scales 2,1,0.5
--scale-group resize-critical
--scale-group upscale,downscale
```

`--scale-group` accepts one group or a comma-separated list. If explicit scales and group scales are both passed, they are unioned in stable sorted order unless `--strict-scales` is passed.

### Accepted baseline support

The accepted baseline represents the current approved performance state for the subject.

```sh
--baseline area-prod-accepted
--save-baseline area-prod-accepted
--replace-baseline area-prod-accepted
--no-run
```

Rules:

- `--save-baseline` fails if the baseline already exists unless `--force` is passed.
- Every measured `perf` run writes a latest-run cache for its command/domain.
- `--replace-baseline NAME` replaces accepted baseline `NAME` from that latest-run cache, then immediately performs a fresh run compared against the replaced baseline.
- `--replace-baseline NAME --no-run` only replaces the accepted baseline from the latest-run cache and exits.
- The harness does not decide whether a change is accepted; acceptance is a human/workflow decision made before invoking `--replace-baseline`.
- Accepted baseline comparisons are pass/fail gates when `--accept-if` or regression flags are set.

### Oracle and spec baseline support

`perf` supports an oracle subject so one run can do correctness verification and performance comparison against the oracle without invoking `comp`.

```sh
--oracle spec:resize:area:scalar
```

When `--oracle` is provided:

1. the oracle subject is included in the measured subject set if not already present
2. every non-oracle subject is checked against the oracle across all fixtures/scales before timing starts
3. the summary table includes an `oracle` comparison column

`perf` also supports saved spec baselines for contextual comparison:

```sh
--spec-baseline spec:resize:area:scalar
--spec-baseline area-spec-accepted
--save-spec-baseline area-spec-accepted
--replace-spec-baseline area-spec-accepted
```

`--spec-baseline` accepts either:

1. a subject ID, in which case that subject is measured over the same case matrix during this run, or
2. a named baseline, in which case stored results are loaded.

Spec/oracle comparison is contextual by default, not a pass/fail gate. It becomes a gate only with explicit spec/oracle acceptance flags. Oracle baselines are auto-created before the normal run when missing or older than 24 hours. If no accepted baseline is specified, perf compares against the most recent compatible previous run.

### Acceptance flags

Initial accepted-baseline gates:

```sh
--fail-on-regression-percent 3
--accept-if "median <= accepted * 1.03"
```

Initial spec-context flags:

```sh
--warn-if-slower-than-spec-percent 400
--fail-if-slower-than-spec-percent 500
```

Expression language may be minimal in v1. The implementation may start with explicit flags and add expression parsing later.

### Runtime logging and perf table

Terminal output should be Criterion-esque and include:

1. bench type, domain, subjects, scales, config, oracle/baseline, and discovered fixtures
2. correctness checks across all fixtures/scales when `--oracle` is provided
3. per-case live warmup/calibration logging with in-place batch-size updates
4. Criterion-style per-case measurement logging with:
   - `time: [mean-stdev mean mean+stdev]`
   - dimmed interval bounds and uncolored center estimate
   - `thrpt` in output MPix/s using the same interval timing
   - live in-place measurement progress updates; detailed stats are rendered from final samples so reporting work does not perturb measured batches
   - horizontal percentile table below the estimate block
   - compact sample stats below percentiles
5. final summary table with accepted baseline, oracle, and spec comparisons when available

Green is reserved for improvement/faster; red is reserved for regression/slower. Correctness and structural status logs should stay neutral.

```text
Benchmarking prod:resize:area:tiled · celeste-960x540-0.5x
  warmup: 1.00s (batch size: 512, taking 10.04 ms)
  measure: 500/500 smp | 5.00s/5.00s | 256000 iter
    time:   [2.0ms 2.1ms 2.2ms]
    thrpt:  [235.6 MPix/s 246.9 MPix/s 259.4 MPix/s]
  prct:    p50    p75    p90    p95    p99
          2.1ms  2.2ms  2.3ms  2.4ms  2.6ms
  stat:   mean      mode     stdev
          2.1ms     2.1ms    0.1ms
  range:   [1.9ms 2.7ms]

subject                 case          mean    median  stdev  p95     MPix/s  baseline      oracle        spec
prod:resize:area:tiled  celeste 0.5x  2.2ms   2.1ms   0.1ms  2.35ms  480.1   1.03x slower  3.81x faster  —
prod:resize:area:tiled  celeste 0.25x 1.1ms   1.0ms   0.1ms  1.12ms  510.4   1.10x faster  4.20x faster  —
```

## `tile` command

### Purpose

Sweep tiling parameters for a subject. This is distinct from normal perf because the sweep matrix is about execution policy, not just filter/scale/fixture.

### Required CLI shape

```sh
ditherette-bench tile resize \
  --subject prod:resize:area:tiled \
  --control prod:resize:area:scalar \
  --fixture celeste \
  --scales 0.95,0.75,0.5,0.25 \
  --tile-heights 8,16,32,64,128,256 \
  --workers 1,2,4,8 \
  --iterations 500 \
  --measurement-time 5s
```

### Sweep axes

Initial axes:

```text
tile_height
worker_count
chunking_strategy
fixture
scale
filter
subject
```

Initial chunking strategies:

```text
whole-image
fixed-row-band
dynamic-row-band
```

### Required scalar control

Every tile sweep must include a non-tiled control either explicitly or implicitly:

```text
subject = control subject
tile_height = none
workers = 1
strategy = whole-image
```

If no `--control` is provided, the harness asks the registry for the subject's declared scalar control. If no control is known, the command fails unless `--no-control` is explicitly passed.

### Tile metrics

Tile result rows include normal perf metrics plus:

```text
tile_height
worker_count
chunking_strategy
band_count
scheduler_overhead_ns, if measurable
speedup_vs_control
efficiency = speedup_vs_control / worker_count
```

### Tile table

```text
subject                scale  workers  tile_h  strategy       median  speedup  efficiency
prod:resize:area:tiled 0.5x   1        none    whole-image    8.20ms  1.00x    1.00
prod:resize:area:tiled 0.5x   4        64      fixed-row-band 2.60ms  3.15x    0.79
prod:resize:area:tiled 0.5x   8        32      fixed-row-band 2.10ms  3.90x    0.49
```

### Tile baselines

Tile baselines are separate from perf baselines by role/path:

```sh
--save-baseline area-tiling-v1
--baseline area-tiling-v1
--replace-baseline area-tiling-v1
```

If no named baseline is provided, tile comparisons default to the scalar control measured in the same run.

## `comp` command

### Purpose

Fast A/B comparison between two registered subjects or implementation files/modules. This is the inner perf-loop workhorse.

### Required CLI shape

By subject ID:

```sh
ditherette-bench comp resize \
  --left spec:resize:area:scalar \
  --right prod:resize:area:tiled \
  --fixtures celeste,gradient \
  --scale-group preview
```

By file path:

```sh
ditherette-bench comp resize \
  --left crates/ditherette-wasm/src/spec/resize/scalar/area.rs \
  --right crates/ditherette-wasm/src/prod/resize/scalar/area.rs \
  --fixtures celeste \
  --scale-group preview
```

File paths are resolved through registry metadata. Unknown paths fail with suggestions.

### Correctness default

Default correctness behavior:

```text
left is oracle
right must match left exactly
```

Options:

```sh
--oracle spec:resize:area:scalar
--verify exact
--verify epsilon --max-abs 1 --rmse 0.25
--verify checksum-only
```

`--verify checksum-only` is an explicit escape hatch and should print a warning.

### Comp acceptance

```sh
--fail-on-slower-than 1.03
--accept-if "right <= left * 0.95"
```

A baseline can be used as the left side:

```sh
ditherette-bench comp resize \
  --left baseline:area-prod \
  --right prod:resize:area:tiled \
  --accept-if "right <= left * 0.98"
```

If the comparison result is accepted, update the perf baseline separately with `perf --replace-baseline NAME --no-run` or `perf --replace-baseline NAME`.

### Comp table

```text
case          left median  right median  ratio         correctness
celeste 0.5x  8.20ms       2.10ms        3.90x faster  exact
celeste 0.25x 3.90ms       1.00ms        3.90x faster  exact
```

### Exit codes

```text
0  command succeeded and acceptance gates passed, or no gates were requested
1  command ran but candidate failed acceptance gates
2  invalid command/configuration
3  correctness verification failed
4  baseline mismatch or missing required baseline
```

## Fixtures

The harness supports real and synthetic fixtures.

### Real fixtures

Real fixtures are decoded before timing unless a command explicitly measures decode cost.

By default, the harness discovers and runs every supported image file in `benchmark-fixtures/` at runtime. Supported first-version extensions are:

```text
png
jpg
jpeg
```

`--fixtures` accepts:

```text
- a direct file path
- a file name inside benchmark-fixtures/
- a unique file stem inside benchmark-fixtures/
- a synthetic fixture name
```

Real image fixture names are not hardcoded. Ambiguous stems fail with candidate paths.

### Synthetic fixtures

Required synthetic fixtures:

```text
solid
horizontal-gradient
vertical-gradient
checkerboard
alpha-gradient
impulse
noise-seeded
```

Synthetic fixtures are important for correctness and filter behavior. They should be deterministic and record seed/config in result metadata.

### Fixture fingerprint

Each fixture has a fingerprint used in baseline matching:

```text
real fixture: path + byte hash + decoded dimensions + pixel format
synthetic fixture: generator name + params + seed + dimensions + pixel format
```

## Case expansion

A benchmark case is a semantic input/output operation. For resize:

```rust
ResizeCase {
    fixture: FixtureId,
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    scale_x: Option<f64>,
    scale_y: Option<f64>,
    pixel_format: PixelFormat,
    params: ResizeParams,
}
```

Scale-derived output dimensions should be rounded by a documented policy:

```text
output_width  = max(1, round(source_width  * scale_x))
output_height = max(1, round(source_height * scale_y))
```

The output dimensions, not the scale string, are the source of truth for baseline matching.

## Measurement contract

Every subject/case measurement follows this sequence:

1. Prepare decoded/synthetic input outside timing.
2. Allocate output buffers outside timing.
3. Run verification before timing when an oracle is available.
4. Warm up and calibrate batch size in the same phase. Calibration doubles batch size until one batch reaches `target-sample-time` or the max batch cap. Warmup completes after calibration is known and the first configured warmup target is reached: `warm-up-time` or optional `warm-up-iterations`.
5. Collect timed samples until the first configured measurement target is reached: `iterations` total measured iterations or `measurement-time` elapsed.
7. Consume output using checksum or black-box after each batch to avoid optimizer lies.
8. Record sample statistics and metadata.

### Timing controls

Defaults:

```text
iterations: 500
measurement_time: 5s
warm_up_time: 1s
warm_up_iterations: optional
target_sample_time: 10ms
min_iterations_per_sample: 1
max_iterations_per_sample: configurable safety cap
```

### Sample stats

Record at least:

```text
min_ns
median_ns
mean_ns
p75_ns
p95_ns
max_ns
stddev_ns, optional
samples
total_iterations
iterations_per_sample
```

Derived throughput:

```text
output_mpix_per_s = output_pixels / median_seconds / 1_000_000
source_mpix_per_s = source_pixels / median_seconds / 1_000_000
bytes_per_s       = bytes_read_written / median_seconds
```

## Verification contract

Verification modes:

```text
exact       byte-for-byte equality
epsilon     max_abs and/or rmse threshold
checksum    checksum equality only; smoke mode, not conformance
none        forbidden unless explicitly requested
```

Default modes by comparison:

```text
spec -> prod same filter: exact for u8 unless subject descriptor overrides
float formats: epsilon
external library comparison: epsilon unless exact is known valid
single-subject perf with no oracle: checksum consumption only, verified=false
```

Verification result schema:

```json
{
  "mode": "exact",
  "passed": true,
  "oracle": "spec:resize:area:scalar",
  "max_abs": 0,
  "rmse": 0.0,
  "first_mismatch": null
}
```

On verification failure, timing must not proceed unless `--time-even-if-invalid` is explicitly passed.

## Baseline storage

Baselines live under Criterion-like target artifacts:

```text
crates/ditherette-bench/target/bench/baselines/<role>/<name>.json
```

The canonical artifact contract lives in `crates/ditherette-bench/ARTIFACTS.md`.

Roles:

```text
accepted    current approved performance state
oracle      auto-managed oracle characterization for current config
spec        saved spec characterization
experiment  temporary or exploratory baseline
```

### Baseline file schema

```json
{
  "schema_version": 1,
  "name": "area-prod-accepted",
  "role": "accepted",
  "created_at": "2026-05-20T00:00:00Z",
  "created_by_command": "ditherette-bench perf resize ...",
  "git": {
    "commit": "...",
    "branch": "...",
    "dirty": true
  },
  "host": {
    "os": "macos",
    "arch": "aarch64",
    "cpu_brand": "...",
    "logical_cpus": 10
  },
  "profile": "release",
  "results": []
}
```

### Baseline matching key

A baseline result is comparable only if all matching fields agree:

```text
subject_id
command/domain
filter
variant
fixture_fingerprint
source_dimensions
output_dimensions
pixel_format
params_fingerprint
```

For tile baselines also match:

```text
tile_height
worker_count
chunking_strategy
```

If a key differs, mark result as `missing-baseline` or `incompatible-baseline`; do not silently compare.

## Result schema

Run result files live under `crates/ditherette-bench/target/bench/<command>/<domain>/<bench-key>/<config-key>/[baseline-name/]<fixture>/<scale>/run.json`, with `crates/ditherette-bench/target/bench/latest/` holding only the newest compatible-run cache. Measurement config includes the sampling/cache mode (`--sample-mode throughput|interactive`, `--cache-state warm|scrubbed`, `--cache-scrub-size`, `--inter-sample-delay`) so throughput and interactive-latency runs do not compare as compatible baselines. Contract details live in `crates/ditherette-bench/ARTIFACTS.md`.

```json
{
  "schema": "ditherette-bench-artifact",
  "schema_version": 1,
  "command": "perf",
  "domain": "resize",
  "created_at": "2026-05-20T00:00:00Z",
  "git": {
    "commit": "...",
    "branch": "...",
    "dirty": true
  },
  "host": {
    "os": "macos",
    "arch": "aarch64",
    "cpu_brand": "...",
    "logical_cpus": 10
  },
  "profile": "release",
  "subjects": [],
  "fixtures": [],
  "cases": [],
  "results": []
}
```

Per-result shape:

```json
{
  "subject": "prod:resize:area:tiled",
  "case_id": "celeste-960x540-area-center",
  "fixture": "celeste",
  "filter": "area",
  "variant": "tiled",
  "source": { "width": 1920, "height": 1080 },
  "output": { "width": 960, "height": 540 },
  "pixel_format": "rgba8",
  "params_fingerprint": "...",
  "verified": true,
  "verification": {},
  "checksum": "...",
  "samples": 50,
  "iterations_per_sample": 3,
  "min_ns": 1900000,
  "median_ns": 2100000,
  "mean_ns": 2150000,
  "p75_ns": 2250000,
  "p95_ns": 2350000,
  "max_ns": 2600000,
  "output_mpix_per_s": 480.1,
  "comparisons": {
    "accepted": {
      "baseline": "area-prod-accepted",
      "median_ns": 2040000,
      "ratio": 1.029,
      "status": "slower"
    },
    "spec": {
      "baseline": "spec:resize:area:scalar",
      "median_ns": 8000000,
      "ratio": 0.262,
      "status": "faster"
    }
  }
}
```

## Plans

Commands may be driven by CLI flags or plan files.

Plan file example:

```toml
[suite]
name = "resize-core"
command = "perf"
domain = "resize"

[measurement]
iterations = 500
measurement_time = "5s"
warm_up_time = "1s"
target_sample_time = "10ms"

[baselines]
accepted = "area-prod-accepted"
spec = "spec:resize:area:scalar"

[[fixtures]]
id = "celeste"
path = "benchmark-fixtures/Celeste_box_art_full.png"

[[fixtures]]
id = "gradient"
synthetic = "horizontal-gradient"
width = 1024
height = 768

[[cases]]
fixture = "celeste"
scales = [2.0, 0.95, 0.75, 0.5, 0.25]

[[subjects]]
id = "spec:resize:area:scalar"

[[subjects]]
id = "prod:resize:area:tiled"
```

CLI flags override plan fields unless `--no-cli-overrides` is passed.

## Registry commands

The harness should provide discovery commands:

```sh
ditherette-bench list-subjects
ditherette-bench list-subjects --domain resize
ditherette-bench list-subjects --module spec
ditherette-bench list-subjects --path crates/ditherette-wasm/src/spec/resize/scalar/area.rs
ditherette-bench describe-subject spec:resize:area:scalar
```

Output should include:

```text
subject id
domain/filter/variant
source file
capabilities
supported pixel formats
default oracle
scalar control, if any
params schema
```

## Error handling

Errors should be explicit and actionable.

Common failures:

```text
unknown subject id
subject domain does not match command domain
unknown fixture
baseline missing
baseline incompatible
verification failed
path maps to zero or multiple subjects
subject does not support requested pixel format
subject does not support requested params
```

Exit codes:

```text
0 success
1 acceptance failure
2 invalid CLI/configuration
3 verification failure
4 baseline missing/incompatible
5 measurement/runtime failure
```

## First implementation slice

Build the harness in this order:

1. `ditherette-bench-api` with subject IDs, descriptors, resize contract, and result structs.
2. `ditherette-wasm` feature-gated registry with spec resize subjects only:
   ```text
   spec:resize:nearest:scalar
   spec:resize:area:scalar
   spec:resize:bilinear:scalar
   ```
3. `ditherette-bench list-subjects` and `describe-subject`.
4. Fixture loading for synthetic RGBA8 only.
5. `perf resize` with one subject, no baselines.
6. JSON result output and terminal table.
7. Real PNG fixture loading.
8. Baseline save/load/compare/replace.
9. Spec baseline support inside `perf`.
10. `comp resize` for registered subject IDs.
11. File-path subject resolution for `comp`.
12. `tile resize` once `prod`/tiling subjects exist.

## Open decisions

- Whether `ditherette-bench` should be in the main workspace once a root Cargo workspace exists, or stay independently invoked by manifest path.
- Whether baseline files should live under `bench/results/baselines` or `benchmark-results/baselines` for compatibility with existing repo conventions.
- Whether first version uses `clap` or a smaller custom parser.
- Whether expression acceptance (`--accept-if`) ships in v1 or waits behind explicit regression flags.
- Whether old prototype subjects should be registered through a separate compatibility crate or left out until the fresh crate has `prod` implementations.
