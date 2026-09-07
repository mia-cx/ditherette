# S10 complete reference quantization

Issue [51](https://github.com/mia-cx/ditherette/issues/51).
Branch `impl/v1-s10-quantize`, PR base `impl/v1-s10-base`.
Review [PR 97](https://github.com/mia-cx/ditherette/pull/97), open and non-draft with auto-merge disabled.
Integration base `af9c0d99dd140f3235968d5f481c5a2c879ce46e` contains:

- S07 `7ef52bd2bcaea2774a400875e5c395526bc9b4b9`.
- S08 `d5e2d9761481f7a6b74fb37c7c2f7841570bead1`.
- S09 `d8bcdcdbe8f874eaee65447a640b97483c9cb775`.

## TODOs

- [x] Complete exhaustive color/metric dispatch and audit inherited metric mathematics with independent vectors.
- [x] Compose validated quantize requests with ordered palette matching, alpha handling, and complete-call fixtures.
- [x] Run native and Wasm validation and record the final evidence.
- [x] Rebase onto the latest integration base and file the unmerged stacked PR.

## Prerequisite validation

All three dependency SHAs pass `git merge-base --is-ancestor <sha> HEAD`.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_color_basic --test spec_color_perceptual --test spec_palette`
passes 30 tests. The wasm32-unknown-unknown `cargo check --locked` also passes.

No production modules, benchmark code, or aggregate ledger changes belong to this slice.

## Metric audit

CompuPhase's inherited normalized red/blue weights omitted the byte-domain `255/256` factors.
The primary formula and website agree. The reference now preserves those factors; explicit primary-axis scores test them.

The website's minimum-chroma hue arc differs from the inherited geometric-mean chord metric.
The coordinator approved separate `OklchHueArc` and `CielchHueArc` tags to retain both coherent behaviors.
The existing circular-hue tags keep their chord semantics. Inventory and tagged-boundary fixtures include the added recipes.

The inherited CIEDE2000 mathematics passes 16 published pairs in both directions, including the hue discontinuity boundaries.
No CIEDE2000 code correction or precision change was needed.

Focused validation passes 23 tests across `spec_quantize_dispatch`, `spec_quantize_metrics`, and `spec_contract`.
The complete color dispatcher uses the seven existing forward/inverse helpers directly.

## Complete-call evidence

`quantize::quantize` validates before preparation/allocation, then composes S09 alpha rules with exhaustive matching.
`matcher::PaletteMatcher` exposes original indices and f32 coordinates for later diffusion and mixing references.
Its only search is a full ordered scan; exact ties retain the first entry.

Ten request tests cover all 15 pairs, nonzero grayscale distances, byte midpoint ties, transparent-only output,
darkest fallback, truncation, index 255, fractional threshold precision, compositing, invalid requests, and durable results.
The unequal-chroma fixture selects index 7 for chord distance and index 3 for website arc distance.
The CompuPhase black/red-17/blue-14 fixture selects blue; the inherited wrong coefficients selected red.
Rec.601 and Rec.709 independently calculated red-15/blue-25 fixtures select different indices.

Focused validation passes 25 tests across `spec_quantize_request`, `spec_quantize_dispatch`, and `spec_quantize_metrics`.

## Final validation

Implementation head `6e1e7431` passes:

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked`: 168 native tests, zero failures.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown`: passes.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --all -- --check`: passes.
- `git diff --check`: passes.

These checks ran in this isolated worktree. No benchmark process or browser timing run was started.
The reference still uses f32 working coordinates and metric arithmetic, with S09's f64 byte-alpha threshold/compositing rules unchanged.

The final fetch/rebase found the integration base unchanged at `af9c0d99dd140f3235968d5f481c5a2c879ce46e`.
The 22 focused request, dispatch, and contract tests pass again after rebase.
Validated code and evidence head was `233ae8eecdd3ac74cd664870d41b4530eef2fb5d`.
The final bookkeeping commit changes only documentation; its exact SHA appears in the coordinator's stack ledger.
