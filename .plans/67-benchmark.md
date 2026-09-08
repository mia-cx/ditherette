# S26 field benchmark registration

Issue #67. Native baseline `b87d965d` starts from accepted all-mode matching `0085972a`, not the rejected S25 dispatch candidate.
Join `07ccc24` includes accepted tooling `bf1b887ca90409b909b742e5eb78f97ab00f5bbd`, including scoped native/public rebuilds.
The join preserves every production byte from `b87d965d`.
Its sole conflict retained S26's existing `rgb8_to_coordinates` addition verbatim.

## TODOs

- [x] Register exact native inverse, field, placement, source-conversion, and complete Processor adapters with typed identities and focused untimed tests.
- [x] Extend the existing public protocol and actual package conformance for perturb/separable outputs after S03's public baseline.
- [x] Declare the fixed 208-worker experiment and conformance fixtures, validate without measurements, and hand off clean checkpoints.

## Fixed proposed measurement scope

Two alternating pairs, 20 samples, 50 ms warmup, 64×48 varied RGBA, strength 0.7, and the S24 ordered palette64.
Sixteen native components plus nine complete recipes in native/Chromium/Firefox/WebKit total 52 cases and 208 serial workers.
Inverse/threshold components use a 250 ms measurement cap.
Source-conversion, placement, and complete calls use a predeclared 10-second cap to retain enough slow baseline samples.
All scopes are single-call latency of one image or component batch, not throughput or cache-hit evidence.

Components are seven original f32 inverse image exports; Bayer2/4/8/16 and random threshold grids;
Oklab radius1 and Oklch radius2 adaptive masks; sRGB and Oklab source-conversion batches including converter construction.
Random seed is `0x12345678`. Adaptive threshold is 10 and softness is 5.
Inverse inputs come from frozen forward conversion outside timing, retaining original byte alpha.
Fields/masks use existing f32 Scores serialization with distinct semantic identities, not rendered-color metrics.
Wide f64 reconstruction remains part of complete perturb, separate from the f32 inverse controls.

Seven complete perturb recipes cover sRGB/Bayer2/everywhere, linearRGB/random/everywhere,
Oklab/Bayer4/adaptive1, Oklch/random/adaptive2, CIELAB/Bayer8/everywhere,
CIELCH/random/adaptive1, and YCbCr/Bayer16/everywhere.
Two complete separable recipes use Bayer4/Oklab/everywhere to sRGB CompuPhase and random/YCbCr/adaptive2 to Oklch hue-arc.
Both use preserve alpha threshold 0.5. Every perturb and matching setting remains in the identity.
Native complete calls retain Processor validation/copies/owned outputs and destruction.
Public conformance compares frozen output and actual `quantize(perturb(...))` outside timing.

No field optimization or measurement is authorized in this task.
The complete native/public baseline must be validated before any call-owned converter candidate.
Use only the assigned S24 benchmark target for Rust checks; preserve its retained `target/s24-*` evidence.

## Native registration checkpoint

Four discoverable component families accept concrete typed parameters for every registered case.
Reference/production IDs differ only in their prefix:
`color:inverse:f32-image-v1`, `field:thresholds:global-v1`, `placement:adaptive:mask-v1`, and `color:source:construction-inclusive-v1`.
The inverse adapter invokes the seven existing image exports with frozen-forward coordinates and original byte alpha.
`ColorInverse`, `FieldEvaluation`, and `PlacementMask` identities extend the shared verifier without a second output model.
Field/mask values use exact f32-bit Scores serialization. Source conversion uses existing packed Color output without fabricated rendering.
Component dispatch, input preparation, and output storage allocation are untimed.
Each production component invocation makes its completed output observable.

Complete production IDs are `prod:perturb:request:processor-v1` and `prod:dither-and-quantize:request:processor-v1`.
They call the actual Processor with copied input and durable native result boundaries.
Processor initialization and mechanical request mapping precede timing; per-call allocation, copies, and result destruction remain inside.
These native boundaries are not a claim about JavaScript allocation cost. Actual public calls remain separate.
The existing deterministic native before/after verification policy remains unchanged.

Validation used the assigned S24 benchmark target:

```sh
cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --test field_adapters --test metric_scores --test quantize_adapters --test reference_subjects
cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --examples
```

All 16 tests pass: four new field adapter tests, four metric tests, three quantize tests, and five reference tests.
The old reference count changed from 19 to 23 for the four new callable families.
New fixtures check seven inverse/conversion spaces, five fields, two placement spaces, complete cross-space calls, alpha, and settings identity.
Both crate formatting checks and `git diff --check` pass.
Production/spec/image bytes still match native baseline `b87d965d`; no mathematical implementation changed.
No measurement or benchmark artifact preparation ran.

## Public protocol checkpoint

The public protocol now calls actual package `perturb` and `ditherAndQuantize` methods.
Their typed identities equal the native complete-call identities, including every field and matching setting.
Perturb responses require RGBA8; separable responses require indexed bytes and complete palette/warning metadata.
Unsupported TypeScript field recipes fail validation before running a worker.

`field_conformance` exports twelve small frozen fixtures through the shared reference registry.
They cover the nine complete benchmark recipes and three palette warning cases.
The existing public conformance suite requires `DITHERETTE_BENCH_FIELD_FIXTURES` alongside its quantize fixtures.
For each engine, it checks every field fixture with primed and fresh instances.
Ten actual `quantize(perturb(...))` compositions verify bytes, palette metadata, transparency, and warnings outside all timers.
Retained outputs survive later calls/disposal, inputs stay unchanged, and perturb preserves alpha.

Untimed conformance passed Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
The package tarball came from the already-validated S26 public baseline.
Its SHA256 is `aa7befbef890f47f83831a7f9f735a9d1a9c5067895620cd36d60f369eab29f2`.
The suite also retained all 47 frozen quantize fixtures across fifteen matching modes.
This artifact supports conformance only; measurements require separately prepared revision-bound artifacts.

Validation passed six browser-worker tests, four native field-adapter tests, eleven paired-browser tests,
and 28 Node browser/timing/IPC tests. Both Rust bins/examples checks pass.
No production, spec, image, or landed kernel implementation changed.

## Experiment declaration and measurement handoff

`field_integration_plan` writes either the 25-case native experiment or the nine-case public experiment.
Use the public experiment independently for Chromium, Firefox, and WebKit.
Each case has two alternating accepted/candidate pairs, giving `(25 + 9 * 3) * 2 * 2 = 208` serial workers.
The generator test verifies scopes, sample counts, caps, unique settings, and identical native/public complete recipes.
Declarations and frozen fixtures were generated under `target/s26-protocol-validation/` without collecting samples.

Reproduce declarations with the assigned native benchmark target:

```sh
cargo run --manifest-path crates/ditherette-bench/Cargo.toml --locked --example field_integration_plan -- native NEW_NATIVE_JSON HOST_LOAD_NOTES
cargo run --manifest-path crates/ditherette-bench/Cargo.toml --locked --example field_integration_plan -- public NEW_PUBLIC_JSON HOST_LOAD_NOTES
cargo run --manifest-path crates/ditherette-bench/Cargo.toml --locked --example field_conformance -- NEW_FIXTURE_JSON
```

All generators refuse to overwrite existing output paths.
The component SourceConversion control keeps the actual per-pixel `rgb8_to_coordinates` helper unchanged.
Its per-pixel converter construction remains inside the batch timer on both revisions.
Only actual perturb/adaptive calculations may use the coordinator's call-owned converter candidate.

The coordinator must prepare accepted/candidate immutable artifacts and supply a separate Wasm build cache before browser preparation.
This worktree owns only the native benchmark cache at `.worktrees/v1-s24-bench/crates/ditherette-bench/target`.
Retain its `target/s24-*` evidence. Wait for coordinator clearance before any measurement.
The coordinator drains agents/builds/tests and holds the shared benchmark lease across all workers and browser children.
No PR, merge, publishing, deployment, or issue closure belongs to this benchmark checkpoint.

The experiment declaration test passed, bringing the focused Rust total to 22.
Rust formatting, JavaScript formatting, `git diff --check`, and bins/examples validation pass.

## Post-measurement threshold verifier repair

The coordinator completed S26 trial 01 and reaped all 208 workers before authorizing this repair.
Its native report rejected five threshold cases because the verifier required a working space for every non-resize operation.
Palette-independent `FieldEvaluation` has no working space. The adapter's identity was correct.
The verifier now exempts that operation alongside resize. Every color-dependent operation still requires its explicit space.

The field adapter test reproduced the exact failure through `verify_three_way` before the fix.
It now verifies all five field grids and both adaptive masks, and rejects a one-bit score difference.
The shared verifier test confirms the space requirement for all eight color-dependent operation tags.
Validation passed 17 focused field-adapter, verification, and metric-score tests.

The opt-in `retained_s26_native_results_reverify_without_running_workers` test reads the original prepared manifest,
report, and 100 native result files. It calls the unchanged paired comparison on retained outputs and samples.
An explicit replay passed. All five threshold gates become `Pass`; all 25 native cases pass.
The other twenty case reports remain unchanged. Sample arrays, medians, ratios, and noise gates remain unchanged.
Browser outcomes and candidate selection remain the coordinator's responsibility.

The derived report is `target/s26-protocol-validation/native-reverified.json` in this worktree.
Its SHA256 is `89064e2f3f6c1defe88a31022a1d8f3f271c1420c3f394cd3890643850546992`.
It includes the original 102 input-file digests and verifier/comparison/replay source digests.
The replay refuses to overwrite a report or write inside the immutable trial root.
All retained input digests matched after replay. No worker, processing call, or collector ran.

For read-only replay, set `DITHERETTE_S26_RETAINED_ROOT` to the coordinator's `target/s26-trial-01` directory and run:

```sh
cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --test field_adapters retained_s26_native_results_reverify_without_running_workers -- --ignored --exact --nocapture
```

Set `DITHERETTE_S26_REVERIFIED_REPORT` only to save a new derived report outside that root.
Use the assigned native benchmark target. Preserve the original report and all measurement provenance.
