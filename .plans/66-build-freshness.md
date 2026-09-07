# Fresh public benchmark compilation

Root observed a shared target cache reusing accepted Wasm for changed S25 candidate source.
The candidate build reported no crate compilation and retained the accepted Wasm digest.
No measurement used those outputs. Root retains that rejected preparation attempt separately.

## TODOs

- [x] Force package-scoped release Wasm cleanup before each benchmark package build, with focused command tests.
- [x] Record validation and remaining cache trust for the bounded fix.

The fix owns only benchmark preparation and its tests/docs.
Preserve dependency caches, immutable artifacts, source mtimes, normal package builds, and all semantic code.
Use each variant's pinned Cargo toolchain and exact target directory.
Tests inject command execution; this task runs no real build, cleanup, or measurement.

## Observed failure and controlled rebuild

The coordinator supplied these observations from its separate preparation worktree.
Accepted source `009b9e37f5a306679bb6eb7a8a68819936b05521` compiled scalar Wasm with SHA-256
`620c944d2d593226dd921baeb3db85174568963e741727e7644217254c1c65e4`.
Candidate source `1f0047f6cac9c4b10b648fc7c51fde707ab1ae9d` initially skipped crate compilation and retained that digest.
Both roles shared release target caches. A clean source inventory did not establish fresh compilation.

The coordinator then ran package-scoped release/wasm32 cleanup in both variant caches.
Cargo removed 11 files totaling 14.9 MiB for scalar and 12 files totaling 14.5 MiB for threads.
The candidate recompiled in 2.92 seconds and 3.11 seconds respectively.
Its scalar SHA-256 became `74c665b6995bcb69e62270d19bdca4a7b8c0990cfa07661e6cc292b0163055c4`.
Those elapsed values describe build diagnostics, not processing performance.

Retained directories under `.worktrees/v1-s25-dispatch/target/s25-trial-01` are:

- `accepted-public`, the accepted artifact.
- `candidate-public`, rejected and unmeasured stale output.
- `candidate-public-rebuilt`, the explicit-clean diagnostic rebuild.

No samples ran. The coordinator will prepare final artifacts using the fixed preparation path.

## Scope and remaining trust

`preparePublicBenchmark` now calls `buildFreshPackage` before packing.
For each role, the helper selects the same pinned channels and variant directories as the package-owned build script.
It executes `cargo +<channel> clean --package ditherette-wasm --release --target wasm32-unknown-unknown --target-dir <crate>/target/<variant>`.
Both cleans must succeed before `pnpm --filter ditherette build` starts.
Cargo owns deletion. This does not touch source mtimes, dependency caches, other profiles, or retained artifact directories.
Ordinary package builds retain their existing incremental behavior.

This prevents stale outputs for the local `ditherette-wasm` crate from crossing benchmark roles.
It is not a hermetic rebuild or proof that cached dependencies are unmodified.
Preparation still trusts the pinned toolchain, dependency cache, and exclusive ownership of the build targets.
Concurrent builds against those same targets remain forbidden.
Source inventories and output hashes bind observed bytes, not a cryptographic proof of their compiler derivation.
New mutable local path dependencies would need equivalent invalidation before reusing these caches.

## Validation

The injected-command test first failed because the old path invoked only the package build.
It now checks both exact clean commands, pinned channels, target paths, working directories, and repeated role preparation.
Either clean failure prevents the package build.
No compiler or Cargo clean command runs in these tests.

`node --test scripts/prepare-public-benchmark.test.mjs` passes all four tests after the fix.
`pnpm exec prettier --write scripts/prepare-public-benchmark.mjs scripts/prepare-public-benchmark.test.mjs` formats the two owned scripts.
`git diff --check` passes. No Rust, Wasm, or public package artifact build ran in this fix worktree.

## Native follow-up

- [x] Add guarded metadata commands and package-scoped native preparation with focused command tests.
- [x] Document the failed-closed attempt and validate the tooling-only overlay without builds or measurements.

The coordinator reports another shared-target freshness failure after public preparation was fixed.
Two workers started. Accepted S24 completed 20 samples; the candidate rejected before timing because its embedded revision remained `009b9e37`.
The candidate checkout was clean at `e218e7a60f5a7d21dcd87f8bea8aab674436254b`.
Both workers exited, the scoped process audit was empty, and the quiet phase ended.
The coordinator retains the interrupted attempt and will not reuse its accepted samples.
This follow-up changes tooling only so identical overlays can apply to accepted and candidate production trees.

`scripts/prepare-native-benchmark.mjs NEW_OUTPUT_DIRECTORY ABSOLUTE_TARGET_DIRECTORY` reads the clean revision and complete source inventory.
It runs pinned Cargo with package-scoped release cleanup for `ditherette-bench`, `ditherette-bench-api`, and `ditherette-wasm`.
The following build keeps the recipe `--bins --examples --release --locked` in the same explicit target.
External dependencies remain cached; no immutable artifact directory is a cleanup target.

Both binaries expose `build-info`, reusing `BuildIdentity::current()` and the existing SHA-256 digest type.
The command holds `BenchmarkGuard` without requiring quiet attestation and returns before registry or workload initialization.
Native and browser trial handlers use the same identity constructor without changing timing or processing logic.
Preparation copies both binaries into a new directory, marks them read-only, and validates their metadata and hashes.
It checks source bytes again before writing `build-provenance.json`; failure leaves no successful handoff file.

Validation uses injected commands and temporary fake binary files, never a real native build or measurement.
`node --test scripts/prepare-native-benchmark.test.mjs scripts/prepare-public-benchmark.test.mjs` passes all seven tests.
Coverage includes exact package/target/build arguments, repeated roles, clean failure, stale revision, dirty status, malformed metadata,
digest mismatch, executable mutation during probing, existing output refusal, and withheld provenance after failure.
Rust formatting passes. Rust compilation and actual guarded `build-info` execution remain for the coordinator's fresh artifact builds.
Pinned compiler and dependency-cache trust still apply; metadata validates observed identity, not arbitrary compiler correctness.
