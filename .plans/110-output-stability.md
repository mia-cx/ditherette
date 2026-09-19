# PR 110 output stability correction

Base `7743c2e5`. Review comment `3952636556` identifies transient A/B/A output changes that endpoint checks miss.
The reproduction uses fake clocks and fake operations. Sixteen calls contain B, but the old collector returns final A without a marker.
Two observer fixtures fail against the old collectors before this correction.

## Measurement protocol change

Every warmup result, measured call, throughput iteration, and initialization probe now reaches an observer outside operation timers.
Throughput retains the calibrated number of calls under one unchanged batch timer. It writes each returned reference into preallocated slots.
After the timer, the observer compares every output. The collector clears slots before disposal, including failure paths.
Initialization probes remain outside the initialization timer. Warmup wall-time observations include validation, preparation, and cleanup.

Retained results assume the public contract's owned, durable buffers. The observer rejects repeated batch buffers and aliases of retained evidence.
Benchmark subjects must return independent byte-array buffers of the declared size, without larger backing storage or overlapping retained results.
This catches overwritten shared storage without pretending to recover bytes an invalid implementation already destroyed.
Only the first result and first distinct successor survive observation. Normal final timing output remains separate inside the collector/page.

The cap is 64 MiB of declared typed output buffers plus 1,024 bytes of conservative per-result bookkeeping.
Accounting includes every batch result and two reserved evidence results. Slots allocate before timing.
If calibration exceeds the cap, the collector fails before preparing/timing samples. It never lowers the calibrated iteration count.
Storage shape checks reject results that violate the declared output byte size after the call.
The cap bounds retained results, not JavaScript engine memory, input fixtures, or later JSON evidence serialization.

Keeping batch results changes allocation/GC behavior. These timings require fresh immutable transport snapshots and new artifact identities.
Old timings remain historical; they do not prove this protocol's performance. No measurements run during this correction.

## Evidence and integration

The wire shape remains unchanged. With an instability marker, `unstable_output` holds the first actual result.
`output` holds the first subsequent distinct result, not necessarily the final sampled result.
The worker writes `first-output` and `first-distinct-output` review bundles, then rejects normal trial publication.
Any marker fails the trial, including a change after an exact preflight without `measure_nonexact`.
An ordinary preflight mismatch still performs no warmup or timing.
HTTP and worker response bounds permit two outputs for every trial, so exact-preflight instability retains both images.

This branch starts before indexed quantize protocol integration. S04/root must adapt observer equality and retained-byte accounting for indexed results.
Count index and palette buffer capacities plus warning metadata; preserve exact indices, palette order, transparency, and warning code/message comparisons.
Keep the same first/distinct failure behavior when joining the newer page.

## Focused validation

The JavaScript fixtures use fake clocks, not real processing measurements.
They cover warmup/sample A/B/A, a transient interior throughput result, initialization probes, outside-timer comparison costs, exact batch counts,
the exact retention cap and one-byte overflow, slot cleanup, aliases, and both HTTP directions' existing bounds.
Rust fixtures validate first/distinct evidence, including an initially exact ordinary trial, and reject normal result publication.

Implementation commit: `c8f365bf35d3223ad76c04b7df5c6ce547cec315`.
`node --test scripts/benchmark-public-timing.test.mjs scripts/benchmark-public-browser.test.mjs` passes 22 tests.
`cargo test --locked --manifest-path crates/ditherette-bench/Cargo.toml --test browser_worker --test paired_browser` passes 15 tests.
The Rust check reuses this agent's idle `.worktrees/v1-resize-bench-subjects/crates/ditherette-bench/target` through `CARGO_TARGET_DIR`.
Owned JavaScript/Rust files are formatted and `git diff --check` passes. All processes exited; no actual benchmark ran.
