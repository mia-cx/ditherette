# S20 initial public-call trial budget

Declare this budget before any browser performance measurement.
The purpose is to establish truthful public-call baselines and prove the fresh-pair tooling, not to promote another nearest optimization.
Read [the implementation plan](61-browser-benchmark.md) and the benchmark execution contract before preparing artifacts.

## Cases

Use deterministic packed RGBA8 fixtures with varied RGB and alpha. All resize cases use the exact version-one center anchor.
Both roles receive the same cropped bytes and return independent durable RGBA8 plus dimensions.

| Case | Source → output | Mode | Preparation | Compared implementations |
|---|---|---|---|---|
| Identity latency | 512×384 → 512×384 | One call | Primed instance | Actual TypeScript / public package |
| Identity throughput | 512×384 → 512×384 | Calibrated throughput | Primed instance | Actual TypeScript / public package |
| Reduction latency | 1024×768 → 512×384 | One call | Primed instance | Actual TypeScript / public package |
| Reduction throughput | 1024×768 → 512×384 | Calibrated throughput | Primed instance | Actual TypeScript / public package |
| First reduction call | 1024×768 → 512×384 | One call | Fresh instance per sample | Actual TypeScript / public package |
| Enlargement latency | 256×192 → 1024×768 | One call | Primed instance | Actual TypeScript / public package |
| Unequal axes latency | 1024×256 → 256×1024 | One call | Primed instance | Actual TypeScript / public package |
| Initialization from bytes | No processing | One initialization | Fresh instance per sample | Public package / public package control |
| Initialization from compiled module | No processing | One initialization | Fresh instance per sample | Public package / public package control |

For a fresh processing sample, initialize outside its timer and dispose afterward. Initialization has its own measured cases.
Initialization from bytes includes Wasm compilation; precompiled initialization excludes compilation. Both use already-loaded wrapper modules and local inputs, excluding network transfer.
Browser-internal compilation caches are not reset. A fresh instance does not claim a cold browser or uncached compiler.
After each timed initialization, verify a tiny nearest result and dispose outside its timer.
The two initialization comparisons are repeatability controls, not TypeScript equivalents or optimization claims.
S19 retains no application cache or content digest. Record cache capability `none`; primed instance state is not a cache hit.

## Fixed effort

Run Chromium, Firefox, and WebKit with recorded executable/runtime identities and explicit isolation settings.
Use four alternating accepted/candidate pairs, 100 samples per child, 250 ms warmup, and a 1,000 ms measurement cap.
Throughput calibration targets 5 ms per sample. Each latency sample executes exactly one public call.
The matrix has at most 216 sequential benchmark workers and 21,600 samples. The coordinator owns one exclusive lease throughout.
Prepare both roles from clean builds with immutable asset snapshots. TypeScript and package may share a source revision; they still execute as separate fresh roles.

There is no kernel tuning budget in S20. Keep measured regressions or inconclusive cases visible for later complete-call work.
At most one repeat of the unchanged paired experiment is allowed for noisy/inconclusive evidence.
A transport defect invalidates its run. Preserve artifacts, repair the defect, then rebuild and re-prepare both roles before another quiet phase.
Never reuse historical samples as a fresh accepted role.

## Correctness and resolution

Verify all outputs against the frozen reference before timing. Preserve exact bytes and metadata in the final proof outside timing.
The existing TypeScript center mapper has a potential tie-rounding mismatch at source 2 → output 49, x=24.
A controlled actual-adapter fixture must confirm and retain that mismatch. It is not an exact performance-equivalent case.
Keep the known mismatch disclosed; do not repair TypeScript or change the frozen oracle to make S20 pass.
Other anchors have no TypeScript equivalent and remain explicitly unavailable there.

Tiny inputs remain conformance and timer-quantization diagnostics. They cannot justify invented positive timings or batched latency.
Retain zero samples and observed timer resolution. Insufficient-resolution ratios remain inconclusive.
Before the real trial, all implementation agents and owned compilers/builds/tests must exit. Resume only after every measurement child exits.
