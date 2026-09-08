# S25 weighted and perceptual matching

Issue #66. Base `0ae8b95f5355b5f311b474034faf2f2bfb686eb5`.

## Ownership and reuse

This worktree owns production metric functions, packed cylindrical integration, quantize registrations, and focused native/private/package fixtures.
The coordinator owns benchmark registration, measurements, joins, and PR preparation.
Builds reuse the idle S24 target at `/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target` exclusively.

Landed `prod/color.rs` already provides Oklch/Cielch conversion and shared Cartesian-to-cylindrical arithmetic. Keep those functions unchanged.
The packed path adds only frozen byte-gray and hue endpoint normalization around that existing conversion.
Specialized metrics and CIEDE2000 are missing production implementations. Copy their frozen functions with mechanical module wiring first.
Keep prepared palette allocation, stable scan order, alpha handling, and complete-result publication from S24.
Spec, image, and policy remain unchanged. No optimization or measurement runs in this task.

## TODOs

- [x] Add literal metric baseline and packed cylindrical wiring; validate all fifteen pairs and commit its exact SHA.
- [x] Extend private/public quantize tags using existing bounded ownership; validate types, errors, and durable outputs.
- [x] Build both Wasm variants and validate native/private/package fixtures plus installed tarball in three engines.
- [x] Record evidence and push a clean handoff for coordinator benchmark work.

## Acceptance

All accepted metric tags select their frozen arithmetic and return exact indices, palette metadata, and warnings.
Known CIEDE2000 vectors, hue seams, neutral colors, ties, and memory failures have focused coverage.
The existing five Euclidean pairs and legacy color APIs retain their behavior.

## Native baseline evidence

Exact baseline commit: `17b2bb003a48fe3151dc7277e88f0dd55d8bf87b`. No optimization follows in this task.

Thirteen focused tests pass across `prod_quantize`, `prod_quantize_allocation`, and `prod_processor_quantize`.
The complete-result matrix covers fifteen matching tags, five palettes, and five alpha policies, including truncation and fractional thresholds.
Packed cylindrical output matches frozen f32 bits for 4,096 colors in each space, including every byte gray.
Known CIEDE2000 neutral/hue-boundary vectors and distinct chord/arc scores pass; duplicate ties and exact preparation budgets pass for every tag.
Existing S24 allocation-failure fixtures still pass. The matching tag increases inline matcher ownership, which existing size-based accounting includes.

`prod/quantize/metric.rs` is the frozen file with only `spec` imports changed to `prod`.
`prod/color/lab_ciede2000.rs` copies its frozen function and helpers, omitting the unrelated unavailable image-writer re-export.
Landed color arithmetic remains unchanged; its barrel only exports the new metric module.

## Public checkpoint

Implementation commit: `a32338e939fee7400d50d4451fe693d94ade62d7`.
The public request/result shape is unchanged from S24. The matching union and runtime validator now accept all fifteen tags.
The private ABI keeps codes 0..4 and appends codes 5..14 for the ten added recipes, in validator order.
Malformed or impossible pairs fail public validation before importing source. Private invalid discriminators retain the existing compact failure convention.
Palette normalization, f64 thresholds, caught copy imports, durable sink publication, and disposal remain the S24 implementations.

`PreparedQuantizer` signatures remain unchanged. `PackedSpace` adds cylindrical spaces; `OrdinarySpace` remains a compatibility alias.
`PaletteMatcher` owns its matching tag and calls `prod::quantize::metric::distance_score` with stable first-index scans.
Euclidean, weighted RGB, chord, and arc scores are squared. CIEDE2000 returns delta E, not its square.
No palette acceleration or extra scratch allocation is introduced.

## Validation

- `pnpm package:build` passes both scalar and threaded release builds, factory generation/staging, and package compilation.
- `CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --features bench-subjects --quiet` passes 309 tests, summed from actual result groups.
- `node --test crates/ditherette-wasm/tests/private_processor.mjs crates/ditherette-wasm/tests/private_quantize.mjs` passes nine tests, including all private matching codes and repeated caught failures.
- `pnpm --filter ditherette test:interface` passes 21 tests and the public type fixture.
- `DITHERETTE_TEST_WEBKIT_EXECUTABLE=/tmp/ditherette-webkit-libs.2dS6Yu/webkit pnpm --filter ditherette test:browser` passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, plus the parent test.

The installed artifact exercises all fifteen pairs, neutral colors, hand-calculated weighted winners, complete metadata, and durable ownership.
WebKit runtime provenance remains the unchanged private setup recorded in `.plans/60-browser-runtime.md`.
Both release targets point to the exclusively owned S24 target subdirectories; no shared target with another agent is used.
The temporary top-level target symlink was replaced by an ignored target directory containing those release-target links.
No build artifacts were deleted.

Formatting, `git diff --check`, and the separately trusted S18 freeze guard pass.
The guard reports revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

All owned builds, tests, and browser children exited. No benchmark, PR, publication, or rollout ran.
Remaining issue work belongs to the coordinator: metric/native/public benchmark registration and evidence, executable inventory integration, and an unmerged PR.
