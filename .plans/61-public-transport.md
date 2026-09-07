# #61 Public browser transport

## Scope

Measure installed-package calls and the real website TypeScript nearest implementation.
The paired protocol owns validation and acceptance. The Rust worker owns reference output and artifact snapshots.

## TODOs

- [x] Add the TypeScript adapter and deterministic timing tests, including zero samples and per-sample preparation.
- [ ] Add the immutable-asset browser transport using the shared trial schema and existing resource cleanup.
- [ ] Verify real TypeScript output and installed-package calls without running measurements; record runtime provenance and handoff.

## Constraints

No benchmark runs during implementation. Fake clocks verify timing mechanics.
S19 has no application cache or hashing. Fresh instances and primed instances describe allocation/runtime state only.
Initialization excludes module loading and network fetch. Bytes mode includes compilation; compiled mode excludes it.
The TypeScript adapter preserves its existing nearest rounding, including the known 2→49 mismatch at x=24.
Only center-aligned TypeScript nearest is available. Identity calls include the durable byte copy within the measured operation.

## Validation

TODO 1 passes six deterministic Node fixtures and focused TypeScript checking.
The offline compiler emits the six real adapter/source modules, with explicit `.js` imports and compiler/input hashes.
Warmup uses wall time including preparation/disposal, with a bounded stalled-clock failure. Throughput calibration uses operation time only.
No real operation timing runs during these tests.
