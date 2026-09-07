# Budgeted native resize subjects

Base `b52d1c8b` plus S22 native checkpoint `53eaf013fed6d00b623b667bad2f396f34de4429`.
The prerequisite join preserves the base's completed S21 evidence document.

## TODOs

- [x] Register eight per-call budgeted subjects and verify exact correspondence with landed production.
- [x] Record native/Wasm compilation and formatting checks; push the clean handoff.

## Timing scope

Each native invocation validates borrowed views, prepares fallible production plans, reserves and fills scratch,
executes the landed kernel, and drops temporary ownership. The existing native caller preallocates image output.
Identity calls retain the landed copy fast path. No public JS boundary, input copy, application cache, or JS-owned output allocation is included.
The developer adapter uses `CapacityBudget::new(u64::MAX)` and translates preparation failures into `BenchSubjectError`.
Existing accepted production subjects and their default oracles remain unchanged.
Fixed Lanczos preparation retains the production const-radius dispatch.

No measurements run here. The coordinator owns experiment generation, fixed budgets, and exclusive measurement clearance.

## Focused correctness

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --features bench-subjects --test bench_resize_budgeted` passes all three tests.
The matrix compares 576 complete RGBA8 outputs against existing production subjects across eight shapes and all nine anchors.
It includes mixed alpha and hidden RGB, exact integer area paths, identity, odd dimensions, single-axis cases,
and scale-aware convolution's large-source x-then-y dispatch. Source bytes remain unchanged.
Registry checks verify all eight explicit oracle IDs and the real adapter source path.
Invalid input views return adapter errors without writing output.

`cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --features bench-subjects --target wasm32-unknown-unknown` passes.
`cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` passes.
Implementation commit `9350a041` changes only benchmark registration, its private adapter, dedicated tests, and this record.
The prerequisite merge changes no frozen spec, image, or policy bytes. No production or public binding changes were authored here.
