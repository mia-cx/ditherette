# S28 literal native diffusion baseline

Issue #69. This bounded phase copies missing diffusion semantics before any ring-buffer optimization or public integration.

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

## Deferred scope

Three-row scratch, capacity-accounted preparation, public methods, benchmark registrations, measurements, and the S28 PR remain unfinished.
The literal baseline must be recorded and benchmark subjects registered before optimization starts.
The coordinator owns integration/tracking and exclusive measurement clearance. S26 source and artifacts stay untouched.
