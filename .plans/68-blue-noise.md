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
- [x] Build both variants and validate installed-package behavior in three engines; record clean handoff.

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

## Validated package checkpoint

Code head `3debdca0df68625195619dee5f460ac135baa446` follows literal baseline `ea47c9a8d16f2d6638530c54dfe29990da13e699`.
Branch `impl/v1-s27-blue-noise` starts at join `bcf123e313290177d47ecb60afcfa0e752ed3d2e`.
The join contains S26 public head `089251287e387cb575e22e8993d8989a371a089d` and S25 PR #114 head `3a9db011207a44f230ae519edc93c900a747c021`.
Both dependency ancestry checks pass. Later evidence-only commits do not change the validated code.

Both `pnpm --filter ditherette-wasm build:scalar` and `build:threads` pass using individual target links.
`node packages/ditherette/scripts/stage-wasm.mjs` stages the generated artifacts before interface and browser tests.
`pnpm --filter ditherette check`, Rust formatting, and `git diff --check` pass.
Frozen spec, shared image storage, and guard policy have no diff against the literal baseline.

The installed tarball passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each engine checks 110 frozen vectors, 1,650 exact compositions, and five caught field-copy failures.
The existing resize, quantize, inert-import, scalar-only loading, isolated-instance, and artifact-layout checks also pass.
The tarball contains no Rust sources or generation/build/test directories.

Browser command:

```sh
DITHERETTE_TEST_WEBKIT_EXECUTABLE=/home/mia/mia-cx/ditherette/.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit pnpm --filter ditherette test:browser
```

The retained WebKit launcher replaces the deleted historical `/tmp` launcher without changing its hardlinked library aliases.
Scalar Wasm is 241,203 bytes, SHA-256 `899de90823e7a85662df70385b35c8e248b03beeb050e674f219fb00f98a83fb`.
Threaded Wasm is 329,487 bytes, SHA-256 `e61644a537efd65670c943926e6d69e7cc926a92de633da97fd71de860e50fcd`.
These builds validate package artifacts; S27 adds no threaded field scheduler.

No benchmark or optimization runs occur in this handoff. S27 still needs field/public-call measurements before a PR.
The coordinator owns the parent update, any required restack, measurement lease, benchmark evidence, and PR readiness.
No merge, package publication, deployment, release tag, or issue closure occurs.
