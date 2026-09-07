# S10 complete reference quantization

Issue [51](https://github.com/mia-cx/ditherette/issues/51).
Branch `impl/v1-s10-quantize`, PR base `impl/v1-s10-base`.
Integration base `af9c0d99dd140f3235968d5f481c5a2c879ce46e` contains:

- S07 `7ef52bd2bcaea2774a400875e5c395526bc9b4b9`.
- S08 `d5e2d9761481f7a6b74fb37c7c2f7841570bead1`.
- S09 `d8bcdcdbe8f874eaee65447a640b97483c9cb775`.

## TODOs

- [x] Complete exhaustive color/metric dispatch and audit inherited metric mathematics with independent vectors.
- [ ] Compose validated quantize requests with ordered palette matching, alpha handling, and complete-call fixtures.
- [ ] Run native and Wasm validation, then file the unmerged stacked PR.

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
