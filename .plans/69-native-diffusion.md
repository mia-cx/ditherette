# S28 literal native diffusion baseline

Issue #69. This bounded phase copies missing diffusion semantics before any ring-buffer optimization or public integration.

## Prerequisites

Start from S26 public baseline `089251287e387cb575e22e8993d8989a371a089d`.
Join validated S25 delivery `bf1b887ca90409b909b742e5eb78f97ab00f5bbd` before copying diffusion.
Duplicate-history conflicts retain S26's packed forward adapter and shared private quantize parser extraction.
Both conflicted files match the S26 parent after resolution. The selected S25 matcher remains unchanged.
The join changes no production or package processing bytes from S26; only inherited documentation and benchmark evidence are added.

The prerequisite join passes 13 scoped field/processor native tests and wasm32 cargo check using the assigned S24 Wasm target.
Full cargo fmt check exposes an inherited module-order difference in src/bench_subjects.rs. That coordinator-owned file is outside this task.

## TODOs

- [x] Inspect existing production helpers, resolve and validate the prerequisite join, and record ownership.
- [ ] Copy frozen diffusion and missing coordinate helpers, with explicit mechanical constructor/import substitutions.
- [ ] Compare all four kernels, both feedback modes, all fifteen metrics, scan orders, alpha policies, placement, failures, and complete metadata against frozen references; commit the literal baseline.

## Copy boundary

No existing production diffusion kernel, error ring, or generic coordinate-dither helper was found.
The frozen module supplies all taps, full-image f32 work, finite checks, scan order, sinks, and both feedback recipes.
Existing production palette preparation, matcher preparation, metric functions, packed conversion, and placement already cover those semantics.
The two frozen constructor calls use private baseline helpers invoking those landed preparation APIs with an unbounded u64 budget.
That adapter is baseline-only, not a public memory guarantee. Full-image allocations remain the literal reference behavior in this phase.
Missing generic coordinate helpers are copied verbatim into a private diffusion common module.
Neither palette/matcher implementations nor landed color, resize, spec, image, or freeze policy files change.

## Deferred scope

Three-row scratch, capacity-accounted preparation, public methods, benchmark registrations, measurements, and the S28 PR remain unfinished.
The literal baseline must be recorded and benchmark subjects registered before optimization starts.
The coordinator owns integration/tracking and exclusive measurement clearance. S26 source and artifacts stay untouched.
