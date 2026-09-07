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
Public Processor/Wasm/package dispatch and benchmark registration are later integration work owned by the coordinator.
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
Public dispatch and benchmark registration remain outside this native handoff. No benchmark ran.
