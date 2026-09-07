# Fresh public benchmark compilation

Root observed a shared target cache reusing accepted Wasm for changed S25 candidate source.
The candidate build reported no crate compilation and retained the accepted Wasm digest.
No measurement used those outputs. Root retains that rejected preparation attempt separately.

## TODOs

- [ ] Force package-scoped release Wasm cleanup before each benchmark package build, with focused command tests.
- [ ] Record validation and remaining cache trust, then push the bounded fix.

The fix owns only benchmark preparation and its tests/docs.
Preserve dependency caches, immutable artifacts, source mtimes, normal package builds, and all semantic code.
Use each variant's pinned Cargo toolchain and exact target directory.
Tests inject command execution; this task runs no real build, cleanup, or measurement.
