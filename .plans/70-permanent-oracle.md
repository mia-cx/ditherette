# S29 permanent oracle integration

Both S29 roles join final S27 parent `518a4f6d6c40f76928a13dc38f71866605e671bc`.
The permanent frozen-only Wasm protocol and immutable snapshots come from `a9327c68560b654b912977102e9fb8036f5f9eeb`.
Clean integration checkpoints are literal `f45f794f02194f66552aa6036128b9fbf702efb8` and candidate `169c609e77b58eafc77c8d9d11d5b5cdd8ff8e19`.
The candidate still changes only its target-conversion call and read-only prepared converter accessor.
S28 diffusion production remains a separate branch. Its future oracle discriminant stays supported.

## TODOs

- [x] Join the permanent identified oracle, snapshot protocol, and settled S27 dependency in both roles.
- [x] Export complete identities for all 367 retained requests and verify independently executed native oracle output.
- [ ] Run all 367 permanent frozen-Wasm identities and package calls in each browser for both roles.
- [ ] Join the coordinator's shared-output rejection, prepare clean fresh paired roles, record evidence, and drain.

## Exactness and provenance

`yliluoma_conformance` exports typed public operations, complete CaseIdentity, original source bytes, and independently executed native reference output.
It checks every native result against the retained pre-integration fixture before exporting.
The installed-package test requires `DITHERETTE_BENCH_ORACLE` and `DITHERETTE_BENCH_YLILUOMA_FIXTURES` from the preparation script and exporter.
The test verifies every oracle manifest input and output digest against current source and actual compiled artifacts.
It then executes the real `frozenBrowserReference` protocol in 367 disposable contexts per engine, before any package initialization.
Every returned identity must match. Every permanent Wasm output must match the previously independent frozen-Wasm fixture.
Package and actual JavaScript-adapter calls consume those newly executed references, not handwritten expected values.
Native/Wasm diagnostic differences must remain exactly cases 236, 248, 251, 254, 257, 260, and 263.
`DITHERETTE_BENCH_ORACLE_EVIDENCE` retains each engine's manifest, tarball digest, identities, native/Wasm outputs, and differences.

The first diagnostic run used JSON serialization equality, which falsely treated object-key order as a semantic difference.
The comparison now uses structural equality. No output arithmetic, tolerance, or frozen fixture changed.
No timer collector or measurement runs in this validation phase.
