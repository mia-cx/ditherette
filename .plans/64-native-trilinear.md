# S23 native trilinear

Base `b52d1c8b7d67dfa2cc0900c05583a6926b835762` contains the restored kernels and public S21 integration.

## TODOs

- [x] Commit a literal frozen trilinear dependency closure with exact native comparisons. Five manifest hashes and two focused tests pass.
- [ ] Add fallible preparation before source copy and allocation-free execution, preserving exact mip/LOD/storage rounding.
- [ ] Verify shapes, anchors, memory failures, compiler targets, and trusted freeze guard; push and drain.

## Scope

Production trilinear is missing. Its exact f64 area and bilinear dependencies stay private to its module.
Landed fractional resamplers use f32 accumulation and can change rounded mip bytes; they cannot replace these exact steps.
Reuse the landed bilinear anchor types and shared allocation helper. Existing area/bilinear implementations remain untouched.
Public Processor/Wasm/package dispatch and benchmark registration are later integration work owned by the coordinator.
No benchmark, cache, or kernel optimization runs in this task.

Bounded preparation reserves complete lower/upper chains, rounded level outputs, and channel scratch before reading source pixels.
All reserved buffers coexist until preparation is dropped. The budget reports that lifetime explicitly, without assuming future cache reuse.
