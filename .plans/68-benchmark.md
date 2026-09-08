# S27 blue-noise benchmark baseline

Issue #68. Validated blue-noise parent `e4a44b718d49c48902806c0a35726ca1acac06ef`
joins accepted S26 delivery `eeb0ba13ec452a26780400b6b7f569c3f7730578` at `28c745a0`.
Documentation-only join `d99ee390` also includes delivered S26 PR #115 head `bb36452ca831bad485f924e7ee007f9bbdb1cb0d`.
Mechanical conflict resolution preserves every blue-noise runtime, frozen spec, shared image, Wasm interface, and package byte.
S26 contributes its benchmark adapters, repaired field verifier, and completed baseline-retention evidence.

## TODOs

- [x] Register the actual fixed blue-noise lookup and complete Processor/public adapters; verify exact frozen outputs through the full verifier.
- [x] Declare 68 serial workers and export matching installed-package fixtures; pass untimed native checks.
- [x] Commit clean code and prepare independent fresh native/public role builds with all jobs drained.
- [ ] Resolve the cross-target frozen-reference assumption below before passing public conformance or preparing immutable pair snapshots.

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

## Prepared builds and exact-conformance hold

Clean code checkpoint `e395b16241f26443a43a049a0fbefb7166215fb8` independently rebuilt both roles.
Retained paths are relative to `.worktrees/v1-s27-bench/target/s27-preparation/`:

- `accepted-native/` and `candidate-native/` contain fresh worker/coordinator binaries and build provenance.
- `accepted-public/` and `candidate-public/` contain fresh scalar/threaded packages, installed consumers, bundle sources, and build provenance.
- Both `ditherette.tgz` files have SHA-256 `32a844782798e87c8e561fd2ca9dc2fd5d653db94ff65648c14702b92f5ecfe1`.
- `native.json`, `public.json`, and `blue-noise.json` retain the fixed declarations and four native-frozen fixtures.

Untimed public checks expose one native/Wasm difference in the Oklab adaptive2 recipe.
At pixel `(8,26)`, RGBA offset 6792, native frozen and actual release-native Processor produce red 95; both public roles produce 94.
The other 8579 bytes and all three other complete recipes match exactly.
Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 show the same result.
The existing smaller S26/quantize checks passed before this larger S27 fixture failed.
This failure does not invalidate that smaller scope, nor does it establish exact conformance for this larger scope.

`{accepted,candidate}-{chromium,firefox,webkit}-diagnostic.json` preserve every full actual/reference output and nine forward-coordinate bit patterns.
`native-diagnostic.json` preserves all four release-native actual outputs and the focal stage comparison.
The diagnostic scripts remain beside these ignored artifacts.
Only the focal pixel's forward Oklab coordinates differ; its eight adaptive neighbors match bitwise.
Input RGBA is `[73,85,65,70]`.
Native coordinate bits are `[1054713084,3167088128,1020538528]`; Wasm bits are `[1054713084,3167088160,1020538512]`.
The adaptive mask remains bit-identical at `1056450327`.
Substituting only Wasm forward coordinates into unchanged native placement/reconstruction moves unrounded red
from `94.50003118629185` to `94.49999517762998`, which explains the final-byte difference.
This localizes the observed mismatch to shared forward color math, without changing the blue-noise lookup.

Keep the native-frozen fixtures and all mismatches intact. No tolerance, fixture replacement, or production fix has been applied.
The coordinator is independently testing the unchanged frozen oracle compiled to Wasm before choosing the reference protocol.
Immutable pair snapshots and all 68 measurement workers remain unstarted.
All preparation/diagnostic jobs are drained; the S23 scalar/threaded caches have returned to coordinator ownership.
