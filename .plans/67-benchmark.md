# S26 field benchmark registration

Issue #67. Native baseline `b87d965d` starts from accepted all-mode matching `0085972a`, not the rejected S25 dispatch candidate.
Join `07ccc24` includes accepted tooling `bf1b887ca90409b909b742e5eb78f97ab00f5bbd`, including scoped native/public rebuilds.
The join preserves every production byte from `b87d965d`.
Its sole conflict retained S26's existing `rgb8_to_coordinates` addition verbatim.

## TODOs

- [x] Register exact native inverse, field, placement, source-conversion, and complete Processor adapters with typed identities and focused untimed tests.
- [x] Extend the existing public protocol and actual package conformance for perturb/separable outputs after S03's public baseline.
- [ ] Declare the fixed 208-worker experiment and conformance fixtures, validate without measurements, and hand off clean checkpoints.

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
