# S28 literal native diffusion baseline

Issue #69. The literal baseline precedes benchmark registration, bounded scratch, and public integration.

## Prerequisites

Start from S26 public baseline `089251287e387cb575e22e8993d8989a371a089d`.
The prerequisite join is `eb80d780249b2728e0b14dcb5904c870286f71f5`.
It includes validated S25 PR #114 head `3a9db011207a44f230ae519edc93c900a747c021` and the S26 baseline above.
Duplicate-history conflicts retain S26's packed forward adapter and shared private quantize parser extraction.
Both conflicted files match the S26 parent after resolution. The selected S25 matcher remains unchanged.
The join changes no production or package processing bytes from S26; only inherited documentation and benchmark evidence are added.

The prerequisite join passes 13 scoped field/processor native tests and wasm32 cargo check using the assigned S24 Wasm target.
The final S25 join includes its benchmark-module formatting correction.

## TODOs

- [x] Inspect existing production helpers, resolve and validate the prerequisite join, and record ownership.
- [x] Copy frozen diffusion and missing coordinate helpers, with explicit mechanical constructor/import substitutions.
- [x] Compare all four kernels, both feedback modes, all fifteen metrics, scan orders, alpha policies, placement, failures, and complete metadata against frozen references; commit the literal baseline.
- [x] Join accepted S26 `3915f60519995cb9087a18b3bfd6bd7220ae804a` and register the full-image native benchmark before optimization.
- [x] Implement fallible three-row scratch with exact arithmetic and width-based capacity checks.
- [x] Expose all four diffusion kernels through the private processor and public package; verify public calls and memory failures.
- [ ] Prepare candidate benchmark subjects for the coordinator's exclusive comparisons.

## Copy boundary

No existing production diffusion kernel, error ring, or generic coordinate-dither helper was found.
The frozen module supplies all taps, full-image f32 work, finite checks, scan order, sinks, and both feedback recipes.
Existing production palette preparation, matcher preparation, metric functions, packed conversion, and placement already cover those semantics.
The two frozen constructor calls use private baseline helpers invoking those landed preparation APIs with an unbounded u64 budget.
That adapter is baseline-only, not a public memory guarantee. Full-image allocations remain the literal reference behavior in this phase.
Missing generic coordinate helpers are copied verbatim into a private diffusion common module.
Its squared-distance import reuses the landed `prod::quantize::metric::euclidean3_squared` with identical arithmetic.
Neither palette/matcher implementations nor landed color, resize, spec, image, or freeze policy files change.

## Baseline validation

Native `prod_diffusion` passes six tests. Its complete-result matrix covers 8,640 combinations of four kernels, two feedback modes, fifteen matching policies, both scan orders, three alpha policies, three placement settings, and four image shapes.
Another 240 combinations match frozen direct quantization at zero strength.
Independent witnesses cover byte-feedback rounding, first ties, and each kernel's raster/serpentine result.
The remaining comparisons cover structured arithmetic/validation failures, transparent sinks and fallback, transparent-only/truncated palette warnings, legacy coordinate taps, padded rows, and three/four-channel input.
The private constructor-adapter unit test checks actual matcher entries against the landed converter across fifteen matching policies, three alpha policies, and mixed/transparent-only truncated palettes.
An external integration test compares that converter's coordinate bits against the frozen matcher for the same colors. This separates the oracle from the production semantic module, as the freeze guard requires.
Frozen `spec_diffusion` passes all thirteen tests. The scalar `wasm32-unknown-unknown` cargo check passes.
Full crate formatting, `git diff --check`, frozen-content verification, and semantic syntax verification pass.
The initial full freeze-guard attempt found the forbidden test-only oracle imports; the corrected checks rerun content and syntax without rebuilding its isolated targets.

Commands use `CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target` and `--manifest-path crates/ditherette-wasm/Cargo.toml`:

- `cargo test --test prod_diffusion --test spec_diffusion`
- `cargo test --lib baseline_constructor_adapters_use_landed_converter_without_reordering`
- `cargo check --target wasm32-unknown-unknown`

No benchmark measurements run in this phase.

## Native benchmark baseline

Literal implementation checkpoint: `91cd93207e1935c53e04cd7b9678cdae742e6f99`.
The accepted S26 join excludes unselected converter candidate `b237b746`.
`prod:dither-and-quantize:diffusion:full-image-v1` calls the literal implementation and retains result allocation/destruction inside timing.
The actual paired native worker accepts typed diffusion settings. Request mapping stays outside timing; identity includes all diffusion, matching, palette, alpha, and placement fields.
The registered subject and timed callable match frozen results for 120 kernel/feedback/matching combinations.
`cargo check --manifest-path crates/ditherette-bench/Cargo.toml --bin ditherette-bench` passes using the assigned target cache.

## Deferred scope

Candidate benchmark protocol completion, exclusive measurements, selection, and the S28 PR remain unfinished.
The literal baseline and native benchmark registration are recorded before optimization starts.
The coordinator owns integration/tracking and exclusive measurement clearance. S26 source and artifacts stay untouched.

## Bounded candidate checkpoint

The native baseline adapter is `c47419e4`. The candidate retains its literal full-image callable for comparison.
`PreparedDiffusion` reuses the landed `PreparedQuantizer` through crate-private matcher/converter accessors.
Three rows start with source coordinates. Each contribution keeps the frozen tap, axis, f64 calculation, and f32-store order.
Scratch owns exactly `width * 3 * 12` bytes with the current allocator; actual capacity is counted after every fallible reservation.
The private `Processor::dither_and_quantize` dispatches diffusion with source/index preflight and complete-result publication only after successful execution.
The candidate's nine native tests pass, including 10,800 frozen comparisons for the ring and complete processor call, exact-limit/one-under checks, allocation/copy/completion/arithmetic failures, recovery, and disposal.
Height growth adds only owned source and index bytes. Work-row capacity remains unchanged across heights 1, 2, 3, 7, and 19.
Four existing field processor tests, scalar Wasm check, full crate formatting, and diff checks pass.
Public package wiring and candidate benchmark registration follow next. No candidate is selected without the coordinator's measurements.

## Public candidate checkpoint

The delivered S26 dependency `bb36452ca831bad485f924e7ee007f9bbdb1cb0d` joins at `c2e3514b9c3709d46db614aab33d95a309b9d833`.
The join's content delta matches only the accepted S26 verifier repair, tests, and documentation. Thirteen relevant benchmark tests pass.
Private family 2 exposes diffusion through the existing borrowed numeric ABI; the signature stays unchanged.
The package adds canonical kernel, feedback, serpentine, strength, and placement validation and publishes the same durable indexed result.
Forty public/private Node tests pass, including 360 generated frozen diffusion vectors, arithmetic distinctions, metadata, exact budgets, copy failures, and recovery.
Scalar and threads package artifacts build using the assigned S24 target's scalar/threads children. TypeScript and declaration fixtures pass.
Installed-tarball conformance passes in Chromium 147.0.7727.15 and Firefox 148.0.2, including all 360 diffusion vectors without cross-origin isolation.
WebKit 2272 cannot launch because the host lacks libwoff2dec.so.1.0.2; its fixture remains registered.
The pnpm automatic dependency-install check rejects borrowed node_modules links; build commands use the installed tools directly. Tarball checks disable that automatic installation with `pnpm_config_verify_deps_before_run=false`.
