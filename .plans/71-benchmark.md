# S30 complete Process benchmark preparation

Issue #71. Branch `impl/v1-s30-bench` starts from native baseline `3335bb69acc6762a30a0b6844aef436c2e6b8de6`.
Its validated prerequisite join is `22b6dd78`. Existing production kernels remain unchanged.

## Scope

Register actual native/public Process calls and equivalent staged resize plus ditherAndQuantize calls.
Reuse frozen Process reference identities with settings `{palette, recipe}` and output dimensions from `recipe.output`.
Require exact process-versus-staged production equality, including metadata and RGBA8 rounding barriers.
Retain inherited optimized-resize/frozen differences as diagnostics; never alter reference bytes or tolerances to hide them.
Keep copies, preparation, output allocation, and destruction inside each timed complete call.
No TypeScript equivalence claim or new optimization is part of this checkpoint.

## TODOs

- [x] Add native subjects, typed settings, and worker registration; verify identity, composition, and invalid requests without measurements.
- [x] Extend public Process/staged adapters, indexed bounds, and protocol tests without package builds.
- [x] Declare a bounded representative full-pipeline matrix with filter-specific targets and fixed budget; commit and report requirements.
- [x] Join the independent runtime and frozen oracle checkpoints; validate their unchanged production bytes and Process wire.
- [x] Validate target-local frozen attribution and actual installed-package adapters in three engines after fresh preparation is assigned.
- [x] Record the fixed measured comparison, raw pair equality, inherited frozen failures, artifact hashes, and held evidence gaps.

Runtime/package code belongs to the runtime agent. Frozen oracle files belong to the oracle agent.
This worktree alone uses `target/compiler`; no shared compiler cache is assigned.
Fresh package validation waits for the runtime integration. Final joins, fresh role builds, and exclusive measurements belong to the coordinator.
Preserve all evidence and follow completed-PR compiler cleanup when active ownership ends.

## Declared experiment

Both roles use the same clean source and built artifact. Accepted calls actual staged production;
candidate calls actual Process. This compares composition overhead, not two source revisions.
Native subjects end in `staged-v1` and `processor-v1`. Public backends are `package-staged` and `package`.
Both workers require exact Process/staged preflight before timing. Paired verification still checks their concrete outputs independently.

| Case | Source | Output | Palette |
| --- | --- | --- | --- |
| nearest, no dither | 512x384 | 257x193 | 16 |
| area, Bayer 4 | 256x192 | 129x97 | 16 |
| bilinear, random | 256x192 | 129x97 | 16 |
| bicubic fixed, blue noise | 129x97 | 65x49 | 16 |
| Lanczos 2 scale-aware, Floyd-Steinberg | 129x97 | 65x49 | 16 |
| Lanczos 3 scale-aware, Atkinson | 129x97 | 65x49 | 16 |
| trilinear, Sierra Lite | 256x192 | 65x49 | 16 |
| nearest bottom-right, Yliluoma 4 | 8x6 | 4x3 | 4 |

The generator fixes source bytes, palette, matte alpha, sRGB matching, and recipe controls.
It declares 128 workers across native and three browsers: eight cases, two pairs, two roles, four runtimes.
Each worker has at most 20 single-call samples, 50 ms warmup, and a 10-second measurement cap.
Public preparation uses primed instances. There is no batching, application cache, or pilot measurement.
The 2 ms target-sample field does not turn single-call latency into throughput batching.
Theoretical total measurement caps are 1,280 seconds, plus bounded warmup and untimed preparation/verification.

## Retained native area diagnostic

All eight final cases match actual staged production exactly. Seven also match frozen Process exactly.
Area differs at seven indexed positions: 2644, 3160, 6772, 7804, 8836, 11932, 12448.
The landed area resize differs from frozen resize in 125 RGBA bytes, each by at most one.
Unchanged frozen dither on landed resized bytes equals both actual production calls exactly.
`target/process-final-fixtures.json` retains original inputs and all three outputs.
`target/process-attributed-fixtures.json` adds full resized buffers and frozen post-resize output without replacing original expectations.

The coordinator explicitly approves `measure_nonexact: true` for area only. The other seven cases retain `false`.
This permits inherited-difference measurements, not selecting a new nonexact implementation.
The actual aggregate gate is `Incorrect` when frozen comparison is nonexact; it must not become `Pass`.
Browser attribution uses separately identified target-local frozen resize and post-resize Process requests.
The browser's actual resized bytes must equal the identified post-resize input. Otherwise retain evidence and stop.
Keep native-versus-Wasm differences separate from same-target production-versus-frozen differences.

## Untimed checks before integration

Two adapter tests cover 49 filter/dither compositions, full identities, recipe-derived dimensions, and invalid settings.
Eleven browser-worker tests pass, including indexed Process output bounds and retained mismatch evidence.
The generator test passes with the fixed 128-worker budget and area-only diagnostic opt-in.
The focused Node Process preflight test passes, including differing-output rejection and resource disposal.
Earlier existing Node timing/public tests pass (28). No measurement worker ran.

## Joined checkpoint

The branch joins oracle `61d338431b5bd7039fa3d1fae4dd44200abdcfc5` and runtime `587339793cf70429b673e888a89d86a332541693`.
Production, private Wasm, public package source, frozen spec, and image files match that validated runtime byte-for-byte.
Only the benchmark subject registration, shared boundary visibility, and new benchmark Process module differ within core source.
The joined checks pass: 11 browser-worker tests, eight frozen-oracle tests, two Process adapter tests, and one generator test.
The Process adapter test also compares all 49 complete identities and native frozen results with the independent oracle.
`cargo check --all-targets` passes for the benchmark manifest. Both modified browser scripts pass syntax checks.

`target/process-oracle-fixtures.json` adds independently identified area resize and post-resize oracle probes.
Its SHA-256 is `63ac50059dfe6fd4681d9afdbe9a1dca8394c19ae561b0d0bf2e3a3d986b6488`.
All original eight fixture identities, inputs, operations, and reference/production/staged outputs remain unchanged.
The installed-browser check writes full references, oracle manifest, package hash, and composition/resize diagnostics before assertions.
Browser Process frozen comparisons and attribution have not run yet. Fresh preparation belongs to the coordinator's next assignment.
The runtime owner's separate 423 process/staged cases per engine are not claimed as benchmark-adapter frozen validation.

Preparation requires one freshly built native executable and one fresh scalar/thread package plus frozen oracle closure.
Both roles may use those same artifacts with their different actual subjects/backends.
Each runtime still receives separate immutable accepted/candidate snapshots and ordinary source/provenance validation.
Use `prepare-native-benchmark.mjs` and `prepare-public-benchmark.mjs` to avoid stale cross-worktree Cargo outputs.
Only this worktree's `target/compiler` is currently owned; assign variant paths before public preparation.
Native/public fixture exporters, final three-engine attribution, snapshots, and source hashes precede quiet clearance.
No timing results, optimization selection, TypeScript equivalence, or full S41 palette/matching coverage are claimed here.

## Final measured handoff

The earlier preparation notes describe their untimed checkpoint, not the final state.
Final source `e5aae7bf0e1761af2f970b6da75d34cf3a813323` completed fresh artifact preparation and three-engine adapter conformance.
Each engine verifies 431 main frozen references, two identified area probes, and 16 exact Process/staged compositions.
The coordinator then runs and reaps all 128 serial workers, collecting 2,504 samples within the declared stopping policy.
All 64 actual pairs have exact case identity and output equality. Implementation identities intentionally differ.
All four aggregate gates remain `Incorrect` solely because area retains its inherited frozen difference.
No new candidate is selected, no confirmed greater-than-10% regression is reported, and inconclusive timing cases remain held.

The [measurement report](71-benchmark-results.md) records timings, proof contexts, size limits, and held S41/S42 work.
Its [machine-readable summary](71-benchmark-results.json) binds raw results, prepared artifacts, events, and conformance evidence.
The retained artifact root is `.worktrees/v1-s30-process/target/s30-trial-01/`.
The report changes documentation only. The coordinator owns compiler cleanup and ledger updates; the runtime owner files the unmerged PR.
