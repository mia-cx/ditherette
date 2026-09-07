# S23 native trilinear

Base `b52d1c8b7d67dfa2cc0900c05583a6926b835762` contains the restored kernels and public S21 integration.

## TODOs

- [x] Commit a literal frozen trilinear dependency closure with exact native comparisons. Five manifest hashes and two focused tests pass.
- [x] Add fallible preparation before source copy and allocation-free execution, preserving exact mip/LOD/storage rounding.
- [x] Verify shapes, anchors, memory failures, compiler targets, and trusted freeze guard.

The separate literal baseline is `fba85a94afca921a9df15286c26ef4bd22281389`.
Its five-file manifest remains historical copy evidence after the bounded preparation changes.

## Scope

Production trilinear is missing. Its exact f64 area and bilinear dependencies stay private to its module.
Landed fractional resamplers use f32 accumulation and can change rounded mip bytes; they cannot replace these exact steps.
Reuse the landed bilinear anchor types and shared allocation helper. Existing area/bilinear implementations remain untouched.
The public continuation joins validated S22 commit `ebbc1c5a32a77f68c2e59595e84a7b227c25c7ee`.
Benchmark registration and measurements remain coordinator-owned.
No benchmark, cache, or kernel optimization runs in this task.

Bounded preparation reserves complete lower/upper chains, rounded level outputs, and channel scratch before reading source pixels.
All reserved buffers coexist until preparation is dropped. The budget reports that lifetime explicitly, without assuming future cache reuse.

## Native integration

`PreparedTrilinear<F>` exposes `required_bytes(source, output)`, `try_new(source, output, anchor, limit)`, `capacity_bytes()`, and `execute(source, output)`.
Preparation uses dimensions only. Execution validates dimensions before writes and performs no allocations.
Every execution refills both source chains, including logical rows from strided views.

Required bytes include the prepared record, nested chain metadata, every mip buffer, rounded outputs, and channel scratch.
Actual capacity uses the allocator-reported vector capacities. Caller-owned source and destination buffers are excluded.
Public integration must add its own image/control capacities before source import, without counting the prepared record twice.
Use this prepared API at that boundary, not the allocating convenience function.

## Validation

- Five focused trilinear tests pass. Coverage includes ten shapes, nine anchors, row padding, hidden RGB, and exact f32 bits.
- Every constructor reservation has an injected failure fixture. Partial chains release their allocations; exact budgets pass and one-under fails before allocation.
- Prepared execution performs zero allocations. Repeated calls refill intermediates, and invalid dimensions leave destination bytes untouched.
- All 292 native tests pass, with zero failures, ignored tests, or measured tests.
- Wasm target compilation, crate formatting, and the separate trusted S18 freeze guard pass.

An independent rounding witness uses four scalar values `[0, 0, 0, 1]` across a 4x1 RGBA image.
The two mip reductions produce `[0, 1]`, then `1`. Averaging only once would incorrectly produce `0`.

Frozen source, image infrastructure, existing area/bilinear kernels, and freeze policy remain unchanged.
No benchmark ran.

## Public continuation

- [x] Add prepared Processor dispatch and algorithm 6 without changing the private signature; support must be zero.
- [x] Verify frozen bytes, anchors, shapes, exact capacity, allocation/copy failure recovery, and durable public output.
- [x] Build both Wasm variants; verify interface, private ABI, installed tarball in three engines, and trusted guard.

The inline trilinear record belongs to `PreparedResize`, which Processor bookkeeping already counts.
Dispatch subtracts that record from native required/actual capacities and restores it only for the standalone constructor's limit.
All heap capacities still preflight before source import. Existing kernel arithmetic stays unchanged.

### Public validation

The public native matrix compares exact frozen bytes for ten shapes and nine anchors.
Exact-limit fixtures derive standalone preparation bytes, subtract its record, then add public bookkeeping and input/output capacities.
One-under rejects without allocating or copying. Every reservation failure releases partial ownership and allows the next call.
Both caught copy failures preserve earlier results and restore the instance.

Commands completed successfully:

- `cargo test --locked --quiet --manifest-path crates/ditherette-wasm/Cargo.toml`: 299 passed, zero failures or measured tests.
- Focused Processor and native trilinear suites: 11 and 5 passed.
- `pnpm --filter ditherette build`: scalar and threaded release builds, factory staging, and TypeScript compilation pass.
- `pnpm --filter ditherette test:interface`: 18 passed, including public type checks.
- `node --test crates/ditherette-wasm/tests/private_processor.mjs`: 8 passed, including repeated trilinear failure recovery without table/page growth.
- Installed tarball conformance: Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 pass, four reported tests.
- `cargo fmt --check`, `git diff --check`, and the separate trusted S18 guard pass.

The tarball command uses `DITHERETTE_TEST_WEBKIT_EXECUTABLE` with the existing alias-preserving runtime at
`/home/mia/mia-cx/ditherette/.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit`.
It covers 27 trilinear cases plus the existing public resize and initialization contract.

The initial odd-mip browser fixture incorrectly expected anchor-independent output.
For `[0,0,255]`, the area mip is `[0,170]`; anchored bilinear outputs are `[43,85,128]`.
Blending against the one-pixel mip `85` at `log2(3)-1` rounds to `[68,85,103]`.
The corrected independent witness passes against the frozen native oracle and all three engines; production arithmetic did not change.

No scalar wrapper signature changed. `privateResize(input, sw, sh, ow, oh, algorithm, anchor, support, sink)` still returns a numeric status.
Trilinear uses algorithm `6`, anchors `0..8`, and support `0`. Public requests omit `support` entirely.
Benchmark registration, measurements, and PR filing remain coordinator-owned. No benchmark ran.
