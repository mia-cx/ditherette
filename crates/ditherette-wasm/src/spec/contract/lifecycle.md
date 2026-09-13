# Processor lifecycle spec

## Scope and inputs

`lifecycle.rs` models initialization choices and one independent synchronous processor instance.
It consumes initialization settings, completed capability attempts, progress records, and timestamps.
It owns no Wasm memory, callbacks, threads, or cache entries.

## Initialization and memory

Threads default to disabled. Disabled uses scalar execution only.
Preferred uses successfully initialized threads when available, otherwise scalar execution.
Required reports `Capability` when threading is unavailable and `Initialization` when threaded initialization fails.
Scalar initialization failure reports `Initialization`.

Memory limits range from 1 byte through 2 GiB, with a default of 1.5 GiB.
Preflight rejects planned capacity above that limit before allocation.
Cache capacity is at most 128 entries and the smaller of 256 MiB or one quarter of the instance budget.
The model excludes caller buffers, returned JavaScript buffers, and fixed overhead from planned private capacity.

## State and progress rules

An idle instance may begin processing or dispose. Processing and callbacks reject recursive begin or disposal with `ReentrantCall`.
Disposal is idempotent; processing a disposed instance reports `Disposed`.

Progress records identify a stage and optional completed/total counts, never an estimated remaining time.
Within one stage, reports occur at most once per 50 milliseconds. Stage changes bypass that interval.
Completed counts cannot exceed total counts. Cache hits may omit intermediate stages.
Disabled progress enters no callback state.

An emitted event enters callback state. A successful callback resumes processing.
A thrown callback returns `Callback`, restores idle state, and publishes nothing.
Completion requires final output readiness and is the last progress event.
Successful finish requires output readiness and, when enabled, a completed completion callback.
Only successful finish authorizes publishing new cache entries.
An expected processing failure restores idle state without publishing partial work.

## Production obligations and non-goals

Production must preserve these transitions and failure categories while implementing actual resources and callbacks.
Returned results survive disposal because their storage is independent of the instance.
This model specifies publication permission, not cache eviction or worker scheduling.
