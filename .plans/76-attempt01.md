# S34 startup attempt 01

The fixed trial stops on a functional threaded-initialization failure, before that worker collects samples.
All 25 started workers are reaped. Maximum live benchmark-worker count is one.
The 24 completed scalar workers retain 480 samples and 2,506 warmup calls.
All recorded output comparisons are exact. No implementation, builds, or tests overlap measurement.

Accepted source is `f55f100fd3efdbe6ae63014b22d0285fa0d64d04`.
Candidate source is `1d1cba8950ab3ffc4064a0a920343e6f92e6a901`.
Candidate package SHA-256 is `01ad17dcf087d1debd7564de9d34acff94805f2034168687ab24e1431412955d`.
Raw requests, outputs, events, stderr, and immutable snapshots remain in
`.worktrees/v1-s34-bench/target/s34-trial-01`.
The failed worker's result file is empty. It supplies no timing or correctness result.

## Completed scalar comparisons

Median initialization times are milliseconds. Imports, initial Wasm fetch, and the output probe are outside the timer.

| Engine | Input | Accepted | Candidate | Gate |
| --- | --- | ---: | ---: | --- |
| Chromium | Bytes | 0.465 | 0.4625 | Pass |
| Chromium | Compiled module | 0.100 | 0.095 | Inconclusive |
| Firefox | Bytes | 5.110 | 4.540 | Inconclusive |
| Firefox | Compiled module | 0.140 | 0.160 | Inconclusive |
| WebKit | Bytes | 1.250 | 1.170 | Pass |
| WebKit | Compiled module | 0.180 | 0.180 | Inconclusive |

These reports do not establish complete release evidence or authorize startup tuning.
No repeated measurements attempt to remove their uncertainty.

## Threaded failure and correction

The first Chromium required-thread worker fails during `preflightOperation`, after the adapter's initial preload/dispose.
Its process exits before measurement. The Firefox threaded cell does not run.
WebKit threaded was already blocked by the independently reproduced atomic-wait termination failure.

A separate untimed repeated-create reproduction captures the original exception from `builder.build()`.
Pinned Rayon calls `Atomics.wait` on the browser main thread, where blocking waits are forbidden.
Cleanup then masks that exception by trying to consume a Rust builder whose borrow remains poisoned by the trap.

S34 corrects its capability check before threaded imports or allocation. Requested threads need a context that permits blocking waits.
Main-thread Preferred selects scalar; Required returns the existing structured capability error.
Scalar main-thread support and the five synchronous methods remain unchanged.
Actual threaded tests and benchmarks move into a host worker. No busy-wait scheduler or helper-pool design is added.
The runtime owner also repairs failed-start builder release at the generated glue boundary.

The frozen lifecycle accepts a capability boolean and already specifies these fallback/error outcomes.
Decisions #24 and #36 keep execution context with the caller. No frozen contract or public option changes.
Fresh functional checks and artifact preparation precede any further measurement. Preserve this failed attempt unchanged.
