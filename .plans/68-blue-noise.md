# S27 scalar blue-noise integration

Issue #68. Base `bcf123e313290177d47ecb60afcfa0e752ed3d2e` contains S26 and selected S25 PR #114.

## Ownership and reuse

This branch owns the production blue-noise lookup/tile, field registration, private/public mode cases, and focused fixtures.
Reuse S26's perturbation, placement, bounded preparation, reconstruction, and separable quantization.
Production currently has no blue-noise implementation. Copy only the missing frozen palette-free field fragments and tile.
The legacy indexed-coordinate adapters are not required by this public path.
Keep generator, spectral analysis, and provenance tooling outside production runtime modules.

Use native target `/home/mia/mia-cx/ditherette/.worktrees/v1-s22-convolution/crates/ditherette-wasm/target` exclusively.
Link only its `scalar` and `threads` children for package builds. The coordinator owns all benchmark targets and measurements.

## TODOs

- [x] Copy literal lookup and tile; verify threshold bits, digest, global rows, and shared perturb output; commit baseline.
- [x] Connect bounded native and private/public mode cases; verify alpha, composition, strict errors, and memory behavior.
- [ ] Build both variants and validate installed-package behavior in three engines; record clean handoff.

No optimization or benchmark runs in this subtask. Subject registration and measurement follow separately before S27 PR readiness.
Frozen spec/image/policy and landed resize helpers remain unchanged.

## Frozen recipe

The 32×32 row-major u16 ranks have little-endian SHA-256 `bcd93746b99ef8ad678ad425f21e1890b4248050b1ea1b382800d7da977e5943`.
The literal lookup computes `(rank + 0.5)/1024 - 0.5` at complete-image coordinates modulo 32.
S14's frozen generation record retains Ulichney void-and-cluster parameters, seed, tie order, and spectral gates.
No regeneration is required to execute or package this fixed asset.

## Literal baseline validation

`cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_blue_noise` passes two tests using the assigned target.
The first checks exact tile-file bytes, canonical rank digest, and 9,801 complete-image coordinate pairs including u32 extremes.
The second checks widths 1/31/32/33/65, tile row wrap, reverse row scheduling, seven spaces, three strengths, and adaptive placement.
Every output matches the frozen RGBA8 oracle, including padding, byte alpha, and hidden RGB at zero strength.
This checkpoint adds no pipeline/public dispatch or optimized lookup.

## Public mode validation

The native pipeline dispatches blue noise through the existing field, placement, reconstruction, and quantization flow.
Private tag 2 requires parameter zero. Public `{ algorithm: 'blue-noise' }` rejects size and seed controls.
The fixed rank tile is the only additional runtime asset; generator and provenance code remain outside published modules.

Focused native tests pass all 12 cases across `prod_blue_noise`, `prod_fields`, and `prod_processor_fields`.
The complete private Node suite passes 11 tests, including strict numeric controls and repeated failure recovery.
`pnpm --filter ditherette test:interface` passes both TypeScript checks and all 24 Node tests.
The fixtures contain 110 frozen vectors, including blue noise in seven spaces and widths 1/31/32/33/65.
All 1,650 field/matching compositions preserve the explicit RGBA8 boundary and complete indexed metadata.
Memory limit, capacity failure, copy recovery, disposal, input ownership, and durable output checks include the new mode.
