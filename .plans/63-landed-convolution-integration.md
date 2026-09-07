# S22 landed convolution integration

Start from restoration `467542f49ce3f600e5b03aeef574a97be554ae15`.
Mia's #108 direction preserves existing production kernels and shared code. Historical copied-baseline records are superseded evidence.

## TODOs

- [x] Add fallible capacity-accounted plans and caller-owned scratch while preserving landed arithmetic and dispatch; prove original-output and failure behavior.
- [x] Validate native/Wasm compilation and frozen independence, commit/push the native support, then await the S21 public integration checkpoint.

## Ownership and seam

This phase owns only convolution/bicubic/Lanczos modules, focused tests, and this record.
S21 owns shared allocation helpers and the initial public processor/package seam. The coordinator owns benchmark code, joins, and PR filing.
Use S21's `CapacityBudget` with compact `Failure`, counting actual vector capacities and rejecting reservation failures.
Mode plans expose conservative metadata/scratch requirements, fallible construction, actual metadata capacity, and scratch length.
Execution accepts caller-owned scratch and uses the landed pixel writers and branch selection.

Fixed Lanczos2/3 keep their const-radius filter dispatch. Scale-aware minification keeps full-image x-then-y scratch above its landed threshold.
Anisotropic and identity branches remain unchanged. No no-scratch fallback, new arithmetic, benchmark, or optimization is authorized.
Public wiring follows only after a validated S21 checkpoint and coordinator-owned join.

## Native support

`BicubicResizePlan` adds `required_bytes`, `try_new`, `capacity_bytes`, and `scratch_elements`.
`LanczosResizePlan` adds the same accounting methods and `try_new2`/`try_new3` constructors.
The Lanczos requirement method takes the radius; its constructors preserve const-radius fixed-policy filters.
Both modules export `resize_*_rgba8_with_plan_and_scratch_into`, returning compact `Failure`.

Requirements include nested vector headers, conservative tap capacity, and the existing full-call f64 scratch.
The caller separately accounts for inline plan storage and input/output allocations.
Construct with one `CapacityBudget`, then reserve and initialize scratch through that same ledger.
Discard the preparation and its ledger together on failure. `capacity_bytes` excludes caller-owned scratch.
Identity plans have no heap storage; existing cached row-band APIs also accept these plans.
The checked full-call API rejects mismatched dimensions, stride, backing length, or short scratch before output writes.

The original one-shot APIs retain their owned scratch path. Pixel writers and reconstruction filters are unchanged.
The full-call kernel only receives optional scratch; its branch order, threshold, and accumulation loops stay unchanged.
Fallible tap construction keeps the original coordinate, weight, clamping, and contribution order.
Legacy row-band execution retains its allocations; it is outside this scalar public-call integration.

Five new tests pass. The matrix covers 378 exact landed-output comparisons across all anchors, both support policies,
identity, odd shapes, single-axis resizes, singleton input, and the large-image separable branch.
Each matrix execution records zero allocations. Fixtures also cover every plan/scratch reservation failure,
successful retry, insufficient budgets, overflow, short scratch, malformed views, and identity row bands.
The four existing independent frozen-oracle and row-range tests also pass.

The landed separable path already documents bounded frozen-oracle behavior for large downscales.
Exact agreement with landed output does not approve a new approximation or establish universal frozen equality.
No benchmark, public package, or browser runtime result is claimed in this native checkpoint.

## Validation

Native implementation checkpoint `e7ec94ec` includes S21 allocation helper changes through `1a92df82`.
Commands run from this worktree, all exit 0:

```sh
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --test prod_resize_convolution --test prod_convolution_allocation
cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects
cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check
node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s22-convolution --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze
git diff --quiet 467542f49ce3f600e5b03aeef574a97be554ae15 -- crates/ditherette-wasm/src/spec crates/ditherette-wasm/src/image tools/spec-freeze
git diff --check
```

Nine tests pass, zero measured. Native and Wasm compilation pass.
The trusted guard passes all isolation checks and retains frozen revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
Its content identity remains `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Next, the coordinator joins this checkpoint with the validated S21 public seam.
S22 then adds public bicubic/Lanczos dispatch and support settings in that joined tree.
Public lifecycle/package conformance and native/browser benchmark evidence remain pending integration.

## Public integration join and TODOs

Coordinator-authorized join `04e54c8` merges exact S21/registration checkpoint `b52d1c8b7d67dfa2cc0900c05583a6926b835762`.
The only conflict was S21's earlier helper plan versus its completed record; retain the completed S21 record.

- [x] Extend prepared dispatch, private ABI, and public validation/types for bicubic, Lanczos2, and Lanczos3; verify native output and allocation failures.
- [x] Build scalar/threaded package artifacts and verify interface, private ABI, and installed tarball across Chromium, Firefox, and WebKit.
- [x] Record checks and frozen identity, push clean checkpoints, and drain all owned processes.

Keep algorithm tags 0/1/2 unchanged; append bicubic 3, Lanczos2 4, and Lanczos3 5.
Add support after anchor in the private ABI. Encode fixed as 0 and scale-aware as 1 for convolution modes.
Other modes require unused support 0; area also requires unused anchor 0 at the raw ABI.
Public requests require explicit anchor and support for convolution. Area/bilinear/nearest still reject support fields.
Private invalid support uses existing `output.resize` failure path, preserving the compact ABI table.
Public validation reports the precise `output.resize.support` path before entering Wasm.
The coordinator continues to own all benchmark registrations, scripts, and measurements.

Prepared dispatch now owns the fallible f64 plans and scratch for all three convolution modes.
Execution failures propagate through the existing compact `Failure` path before publishing a result.
Public types require both anchor and support. Raw tags reject invalid support and unused settings without poisoning the instance.
No kernel file changed after native checkpoint `53eaf013`.

The native public matrix passes 486 exact comparisons against landed output across nine shapes, all anchors, and both policies.
It includes the separable threshold branch, alpha extremes, hidden RGB, and maximum supported source/output side lengths.
Frozen comparison diagnostics found zero differing pixels in this matrix; they do not establish universal exactness.
The shared processor failure fixture now covers each convolution mode/policy, nested metadata, scratch, image buffers, and copy/result failures.
Every failed preparation releases all owned bytes; retry succeeds. Exact-capacity requests pass and one byte less rejects before allocation.

The separate 2×1 half-byte alpha fixture yields 127 for scale-aware Lanczos3 in the landed Wasm path; the other modes yield 128.
Direct legacy `resizeRgba8` confirms this output. The public fixtures preserve it without changing arithmetic or approving a new approximation.

## Public validation

Implementation checkpoint `f2a385621d53493c717161fd7fc2008ef24fb169` passes:

```sh
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects
cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects
cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check
pnpm package:build
pnpm --filter ditherette test:interface
node --test crates/ditherette-wasm/tests/private_processor.mjs
DITHERETTE_TEST_WEBKIT_EXECUTABLE=/tmp/ditherette-webkit-libs.2dS6Yu/webkit pnpm --filter ditherette test:browser
node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s22-convolution --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze
git diff --quiet 53eaf013 -- crates/ditherette-wasm/src/prod/resize/scalar/bicubic crates/ditherette-wasm/src/prod/resize/scalar/lanczos crates/ditherette-wasm/src/prod/resize/scalar/convolution
git diff --quiet 467542f4 -- crates/ditherette-wasm/src/spec crates/ditherette-wasm/src/image tools/spec-freeze
git diff --check
```

All 293 native tests pass; every group reports zero measured tests.
The package builds both scalar and pinned threaded variants. The threaded compiler retains its existing atomics warning.
Sixteen package interface tests and seven private ABI tests pass.
Private ABI conformance includes 54 exact comparisons against the unchanged legacy Wasm entrypoint.
Installed-tarball conformance passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
That suite reports four passing tests including its parent, with 54 convolution anchor/support cases per engine.
Each engine also checks memory-limit recovery, malformed support, alpha output, and durable result ownership without isolation headers.
Prettier checks pass for all eight changed package TypeScript/JavaScript files.
The trusted freeze guard retains the revision and content digest recorded above; spec/image/policy bytes are unchanged.

Initial interface validation caught misplaced negative-type-test directives after expanding the union; those directives now sit on the rejected fields.
The alpha fixture initially assumed 128 for every mode; the legacy check above established the existing Lanczos3 result.
No unresolved check failure remains. No performance trial ran; the coordinator owns fresh artifacts and comparative acceptance evidence.

Public code `f2a38562` and validation record `89f87985` are pushed on `impl/v1-s22-convolution`.
All owned build/test/browser sessions exited. A scoped `/proc` working-directory check found no remaining process in this worktree.
The worktree is clean at handoff. The coordinator can join this branch for fresh native/public comparative trials.

## Final stacked PR validation

Join `d2fabf28364ace91faae84e0794a84bd9ddd2f3d` includes finalized S21 parent `7743c2e5b4a313fa5e7da70b850956b85a880df4`.
The required rebase onto `origin/impl/v1-s21-area-bilinear` used `--rebase-merges` and preserved the exact tree and original checkpoints.
Conflict resolution retains the validated S22 extensions, including support tags and fallible execution.
Compared with coordinator `49157a78`, the join changes only S21's measurement record and its authorized progress row.
Production, package, benchmark runtime, frozen reference, image infrastructure, and guard bytes remain unchanged.

Checks rerun in the final S22 worktree all pass:

- 293 native tests, plus three feature-gated budgeted adapter tests. All report zero measured tests.
- Scalar and pinned threaded package builds, including generated private bindings and TypeScript compilation.
- Sixteen interface tests and seven private ABI tests.
- Installed-tarball Chromium, Firefox, and WebKit conformance, with four reported tests including the parent.
- Rust formatting, diff checks, and the separate trusted S18 freeze guard.

The [retained measurement record](63-measurement.md) describes runtime `1761705e`, not a new trial on the PR head.
Read-only verification confirms all 24 native production pairs are byte-identical and all four S22 reports are complete.
Their counts and medians match the record. Existing strict-reference failures remain visible; public comparisons remain non-equivalent diagnostics.
No measurement or kernel change occurs during PR assembly. The PR stacks on S21 and closes issue 63 only when merged.
