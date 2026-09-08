# S31 preparation-cache reuse inventory

Read-only inventory against native Process baseline `3335bb69`. S31 implementation has not started.
Start from the validated S30 PR head. Read issue #72, decision #37 and its addenda, and the approved PRD.
Re-read the actual joined production code before editing.

## Existing code

Paths below are under `crates/ditherette-wasm/src`.

- `spec/contract/cache.rs` and `cache.md` define typed identities, accounting, scratch-first pressure handling, LRU, pending publication, and failure behavior.
- `prod/quantize/prepared.rs::PreparedQuantizer` owns prepared palette, matcher, and converter. Execution reads this preparation.
- `prod/pipeline/resize.rs::PreparedResize` dispatches all landed kernels. Some variants combine plans with mutable scratch.
- `prod/palette/allocation.rs` and `prod/resize/common/allocation.rs` already provide fallible reservation and actual-capacity accounting.
- `prod/pipeline/processor.rs` owns instance lifecycle but currently retains no preparation or scratch between calls.

## Missing ownership

Implement the frozen control contract in production before optimizing it. Keep frozen files and existing kernels unchanged.
Use one per-instance preparation cache, ready to share its budget and LRU with S32.
Retained entries have both limits: 128 entries and `min(256 MiB, memoryLimitBytes / 4)`.
Idle scratch counts against total memory separately. Drop idle scratch before evicting published entries under total-memory pressure.

Prepared palette keys include palette identity, alpha, and matching. Resize-plan keys include source dimensions and complete output settings.
Neither preparation key has an image parent. Calling method and threads do not affect identity.
Cache misses remain pending until final output completion succeeds. Disposal releases every retained allocation.
S32 owns source-content hashing and cached image/color/indexed stages; do not add those in S31.

## Accounting traps

Palette cache identity binds the first 256 entries and truncation flag. Benchmark case identity deliberately binds the full request instead.
Keep full validation before cache hits, including invalid palette tails and existing error ordering.
Match normalized signed zero and full alpha-threshold precision. Frozen hashing allocates infallibly; production must handle its own allocation failures.

Quantizer capacity includes its inline record; resize capacity currently excludes its inline record.
Reconcile both conventions when moving storage. Count pinned records, pending entries, containers, boundaries, and scratch exactly once.
`PreparedDiffusion` combines a quantizer with width-dependent row scratch. Do not cache that entire record under a palette-only key.
Trilinear preparation contains image-sized mip storage, including level zero. Reuse it only as overwritten scratch, not a cached image result.
Failures discard pending entries and active scratch. Previous hits and permitted evictions need not roll back.
An oversized cache candidate can still complete uncached.

## Focused validation

Verify frozen-key agreement, order/duplicates/transparency/truncation, signed zero, precise thresholds, and complete resize settings.
Check cold/warm exactness across staged and complete calls, including changed source bytes with unchanged dimensions.
Exercise actual overcapacity, both retention limits, LRU, scratch-first pressure, and two preparations pinned by one Process call.
Test allocation/final-copy failures, recovery, isolated instances, and disposal.
Declare bounded cold/warm full-call measurements before running them. The coordinator alone schedules exclusive measurement and compiler cleanup.
