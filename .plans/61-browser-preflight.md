# S20 browser benchmark preflight

This is read-only preparation for [S20](../docs/plans/ditherette-v1/slices.md#s20), issue #61.
Implementation waits for the validated S19 package and its unmerged PR. Start from that delivered commit, not this preliminary join.
Read the approved PRD, `crates/ditherette-bench/EXECUTION.md`, and `PAIRED.md` before implementation or measurement.

## Reusable protocol

`crates/ditherette-bench/src/paired.rs` owns typed cases and fresh-pair comparison.
`paired/coordinator.rs` already snapshots executables, alternates roles, retains raw artifacts, and enforces the lease.
Extend prepared trial dispatch in `src/main.rs` through a browser worker beside `paired_native.rs`.
Reuse `Lease::spawn`, `OwnedChild`, and `scripts/benchmark-transport.mjs` for Node/browser ownership and cleanup.
The chain stays coordinator → benchmark worker → Node → browser, with one benchmark worker at a time.

`PairCase` currently names native center/default identities without executable operation-specific settings.
Add a typed public-operation registration path that later processing slices extend; initially register nearest resize.
Keep existing native evidence readable and reject unsupported combinations instead of reinterpreting them.

## Actual measured boundary

The old browser runner calls `benchmarkResizeRgba8` batches. S20 must call the installed package's `createDitherette` and `resize`.
Each latency sample times exactly one synchronous public call, including its validation, allocations, and boundary copies.
Measure initialization separately and state whether module loading/compilation is included.
Keep calibrated throughput separate from single-call latency.

S19 has no content hash or application cache. Report cache capability `none`, not cache hits.
Fresh and primed instances are different preparation states, not substitutes for empty and warm content caches.
When later slices add hashing, whole-call timing includes it automatically. Do not add benchmark-only hashing and claim public cost.
Provide an explicit later cache-state contract. A cold-cache sample must reset before each measured call, including after warmup.

## TypeScript comparison

Use the actual `src/lib/processing/resize.ts` implementation and its dependency closure.
Both roles start with cropped RGBA8 and return durable RGBA8 plus dimensions.
The TypeScript identity path returns its input, so the adapter must copy that alias inside timing.
Nonidentity outputs already own their bytes; avoid an extra unnecessary copy.
TypeScript nearest supports center sampling only. Record the other eight anchors as unavailable for that comparison.
There is no equivalent TypeScript Wasm initialization operation. Never invent a zero-time result for it.

## Identity and verification

Snapshot/hash the complete served package, Wasm/glue/helpers, browser adapter, and compiled TypeScript closure.
The worker executable's hash alone cannot identify a browser benchmark artifact.
Bind browser engine/version/executable, launch options, Playwright/Node versions, and isolation state.
Verify served asset identities before and after each prepared run.
Reuse S05 input/settings digests, recorded outputs, three-way verification, and mismatch artifacts outside timing.

Legacy `wasm_resize.rs` fixture-name/dimension fingerprints and constant settings identities are insufficient here.
Its user-agent/isolation fields and 32-bit output checksum do not replace exact output and artifact verification.
Extend owned-child transport fixtures to reject malformed results and identity mismatches while preserving cleanup.

## Completion boundary

S19's native pair is not public-package evidence. Prepare and measure fresh browser artifacts after implementation drains.
No browser benchmark runs during this preflight, and no S20 dependency becomes available merely because this file exists.
