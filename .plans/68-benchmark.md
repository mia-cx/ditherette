# S27 blue-noise benchmark baseline

Issue #68. Validated blue-noise parent `e4a44b718d49c48902806c0a35726ca1acac06ef`
joins accepted S26 delivery `eeb0ba13ec452a26780400b6b7f569c3f7730578` at `28c745a0`.
Documentation-only join `d99ee390` also includes delivered S26 PR #115 head `bb36452ca831bad485f924e7ee007f9bbdb1cb0d`.
Mechanical conflict resolution preserves every blue-noise runtime, frozen spec, shared image, Wasm interface, and package byte.
S26 contributes its benchmark adapters, repaired field verifier, and completed baseline-retention evidence.

## TODOs

- [x] Register the actual fixed blue-noise lookup and complete Processor/public adapters; verify exact frozen outputs through the full verifier.
- [ ] Declare 68 serial workers and export matching installed-package fixtures; pass untimed native and three-engine adapter checks.
- [ ] Commit clean code, prepare independent fresh native/public role artifacts, and hand off immutable paths with all jobs drained.

## Fixed scope

The accepted implementation already uses an O(1) 32×32 lookup. No new optimization candidate is proposed.
Both roles independently rebuild the same clean revision to establish the exact baseline and check integration stability.
Measurements remain held for the coordinator; preparation never launches a measurement worker.

Use a varied 65×33 RGBA image to cross two tile column boundaries and one tile row boundary.
Register one native threshold-grid component and four complete recipes:

1. sRGB perturb, everywhere.
2. Oklab perturb, adaptive radius 2, threshold 10, softness 5.
3. YCbCr perturb, everywhere.
4. YCbCr perturb, adaptive radius 1, into Oklch hue-arc matching with S24's ordered palette64.

Every recipe uses the fixed blue-noise field and strength 0.7.
Separable matching preserves alpha with threshold 0.5.
Five native cases plus four cases in each of Chromium, Firefox, and WebKit total 17 cases.
Two alternating accepted/candidate pairs give 68 serial workers.
Each case uses 20 single-call samples and 50 ms warmup, with no application cache.
The threshold cap is 250 ms; every complete call has a predeclared 10-second cap.
The shared full verifier requires exact threshold bits, RGBA bytes, indexed bytes, and metadata.
Public conformance also compares actual `quantize(perturb(...))` outside timing.

## Artifact and ownership limits

Own benchmark adapters, declarations, fixtures, tests, and this plan only.
Keep production, frozen spec, image storage, and public package code unchanged after the mechanical join.
The native cache is `.worktrees/v1-s24-bench/crates/ditherette-bench/target`.
Preserve all retained `target/s24-*` evidence.
For both public role builds, link only local Wasm target `scalar` and `threads` children to
`.worktrees/v1-s23-trilinear/crates/ditherette-wasm/target/{scalar,threads}`.
Return those exclusive caches to the coordinator after preparation and validation finish.
Fresh preparation uses existing source-inventory and build-provenance checks.
No measurement, PR/issue write, merge of a PR, publishing, deployment, or candidate selection occurs in this task.

## Code validation checkpoint

The existing field registry now dispatches BlueNoise to actual production/reference lookup functions.
Complete Processor and public recipes accept the already-delivered blue-noise mode through their existing typed adapters.
No extra runtime implementation or table copy was introduced.
The full verifier passes exact 65×33 threshold and complete outputs, including crossings at 31/32 and 63/64 columns and 31/32 rows.
Native and public identities match, and perturb alpha remains unchanged.

Twelve focused Rust tests pass across blue-noise adapters, field adapters, browser workers, and the fixed matrix example.
The retained S26 replay test remains opt-in and was not rerun.
Twenty-six Node browser/timing protocol tests pass. Rust bins/examples checks and both formatting checks pass.
Declarations and four frozen installed-package fixtures were generated in `target/s27-preparation/` without timing.
Set `DITHERETTE_BENCH_BLUE_NOISE_FIXTURES` to its `blue-noise.json` for the existing public conformance suite.
That suite checks all four new recipes with primed/fresh instances, including actual separable composition and durability.
