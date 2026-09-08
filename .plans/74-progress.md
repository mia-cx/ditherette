# S33 public progress and callback failure

Issue [#74](https://github.com/mia-cx/ditherette/issues/74). Branch `impl/v1-s33-progress`
starts from validated S32 `59b5a004c98c4cff255b30f51025d8ec8143786d` in
`.worktrees/v1-s33-progress`. The coordinator authorized implementation after S32 measurements drained.
S32's cold-cache release hold remains separate; this slice does not retune its cache.

## Contract and reuse

The coordinator's `.plans/74-reuse-inventory.md`, approved execution contract,
S33 slice, decision #24, and frozen `spec/contract/lifecycle.rs` have been read.
Reuse its existing production `InstanceModel`, eight `Stage` tags, `Progress`
counts, and 50 ms gate. Keep frozen `spec/` and shared `image/` unchanged.

`Boundary` and `QuantizeBoundary` already construct durable output before
`preparation::Call::finish`. Add completion delivery between these existing steps,
including early indexed/RGBA cache hits. A thrown callback must leave the call
unsuccessful, drop pending entries, and preserve recovery. Existing TS `#active`
and Wasm `Slot::Busy` guards reject processing/disposal reentry across callbacks.

Use the existing caught scalar/void import protocol and private result sink.
Enable the existing request callback field; do not add public settings or cache controls.
Keep allocation-free status 12 and append `onProgress` to the aligned private error-path table.

Report only executed work. Fused alpha/color conversions do not become synthetic passes.
Quantize and Yliluoma report completed rows in their existing loops. Diffusion keeps
one continuous three-row feedback traversal. Perturb keeps full-source adaptive
neighbors and global random indices. Resize retains its current prepared plans,
caller-owned scratch, arithmetic, and specialized dispatch. Add minimal progress
hooks to those kernels; do not substitute allocating legacy row-band wrappers.
Trilinear counts shared chain construction, sampling, and blending as actual work.

## Ownership and validation

This worktree owns runtime/lifecycle integration, minimal existing kernel hooks,
Wasm/package source, native/private/interface tests, and this plan. A separate
agent owns installed progress fixtures. The coordinator assigns benchmark work
and owns issues, stack records, measurement clearance, and final acceptance.

The only native compiler target is this worktree's new `target/compiler`.
If Wasm builds are needed, use fresh local `crates/ditherette-wasm/target/scalar`
and `target/threads`. No shared symlinks or S32 compiler outputs are reused.
Measurements require explicit coordinator quiet clearance. Routine reviews wait
until the full implementation stack exists.

## Atomic steps

1. [x] Add the allocation-free callback boundary and copied-model progress controller.
   Compare fake-clock schedules and completion permission with frozen lifecycle fixtures.
2. [x] Add countable, fallible hooks to existing kernel loops without changing their outputs or allocations.
   Prove callback-disabled equality and abort behavior with focused native tests.
3. [x] Connect all five pipelines and success-only completion, including image hits.
   Prove no new publication after intermediate/completion failure and successful recovery.
4. [x] Enable typed request callbacks through the caught Wasm boundary.
   Verify callback errors, getters/reentry/disposal, result readiness, and private handle recovery.
5. [x] Join installed fixtures and the assigned overhead protocol; validate fresh artifacts.
   Record exact source/artifact identities, await exclusive measurements, and file an unmerged child PR.

Each completed step is a buildable commit. Count callback/control records in the
existing ownership budget; callback-disabled calls must avoid clock imports.

The first checkpoint adds optional borrowed callback access to both existing
boundaries and reuses `InstanceModel` for gating. Two native tests pass, including
fake-clock event equality against the frozen lifecycle and thrown completion/recovery.
The callback error uses the static failure record instead of allocating diagnostic text.
Pipeline behavior remains unchanged until the later wiring step.

The kernel checkpoint adds row hooks to quantize, perturb, Yliluoma, diffusion,
fractional area, bilinear, and shared convolution. Trilinear counts chain rows,
whole existing sampling/reduction batches, and blend rows. Nearest and exact area
retain their optimized whole-call batches. The 50 ms rule bounds event frequency;
it does not require splitting these optimized batches or impose callback latency.
Convolution counts both source filtering and output rows on x-then-y paths.

All 5 progress fixtures pass. They cover all seven resize policies, one-axis and
integer paths, scale-aware convolution, mip blending, abort, recovery, and unchanged
prepared capacity. The full native `cargo test --locked --tests` suite passes,
including existing independent frozen comparisons. No measurements ran.

Pipeline wiring now covers all five methods and early image hits. Completion
uses the durable boundary result before `Call.finish`; any callback error drops
that result and leaves publication unsuccessful. Disabled calls dispatch through
the existing no-op kernel specializations and never read the clock.
The inline controller record belongs to the existing private capacity budget.

Two focused native fixtures pass across every method. They observe readiness
inside completion, zero retained entries after prepare/intermediate/completion
failure, no image hits from failed calls, recovery, warm-hit completion failure,
and no completion after final-copy rejection. Existing processor (14), Process (6),
and diffusion (10) tests also pass, retaining the physical allocation witnesses.

The wrapper checkpoint enables the existing typed `onProgress` field across all
methods. Validators preserve one raw property read, including Process's composed
validators. The private sink carries a borrowed callback. Caught void delivery
clears failed completion output and maps thrown callbacks to status 12/path 38.
Both active guards remain in force while getters and callbacks run.

Fresh local scalar and threads builds pass. Interface/type tests pass 35/35;
private Wasm tests pass 17/17, including 512 progress failure/recovery cycles with
stable externref live slots, capacity, and Wasm pages. Factory/staging tests pass 7/7.
Existing shared browser fixtures now test malformed callback types instead of the
obsolete unsupported-function expectation. The new installed progress suite owns
positive callback coverage, so accepted S32 conformance remains usable separately.

Final S32 PR #122 at `127a0428a0bfdad7ea3e239e6a96f375449815bb` and installed
fixture head `0bb2552cd3b2797c1560d2de8b96c5c80c48cc15` are joined.

The trusted guard caught a frozen-module reference inside a production unit test.
The independent comparison now lives in `tests/prod_progress.rs` and observes an
actual quantize call against the frozen lifecycle. Production behavior and guard
policy remain unchanged. Initial `20507aa0` artifact preparation had already
started; its jobs finished before this test-only edit. Those artifacts are superseded.

## Candidate validation before measurements

The assigned benchmark protocol at `ec640c6dfc48afcc8fe95edcf1d2fd16c8e74579`
is joined. The full native test suite passes after that join. The relocated
frozen comparison and four remaining progress unit tests also pass.
The coordinator's trusted guard passes at clean source
`4a75d479d38a92c75e8ff4ed96c916fec3aaf8f4`, including isolated native/Wasm
production and frozen builds, plus native threaded production. The frozen digest
remains `17ba3be3`.

Both official preparers completed successfully with that source held fixed.
Their manifests record the same clean revision. Retained artifact directories are
`target/s33-candidate-4a75d479-native` and `target/s33-candidate-4a75d479-public`.
The installed tarball SHA-256 is
`f6e62526cc290d8ca1f9fdcb7fcfc9de39790c1982dab7118adb30a4a84173d2`.
It is byte-identical to the superseded `20507aa0` package. That earlier source
failed isolation and remains unselected regardless of its passing callback tests.

Tests run from `4a75d479` against the exact replacement tarball pass:

- Installed progress and stage-ownership suites pass 8/8 tests.
- Broad installed conformance passes 4/4 tests. Each engine checks 367 frozen
  Wasm Yliluoma vectors and 734 untimed actual benchmark-adapter calls.
- Engines are Chromium `147.0.7727.15`, Firefox `148.0.2`, and WebKit `26.4`.

The broad suite uses the replacement public oracle and an untimed fixture generated
by the official native preparer's `yliluoma_conformance` executable. Tests use
`DITHERETTE_TEST_TARBALL` and its required SHA-256, plus the existing WebKit alias.
This record is a report-only delivery delta after the artifact source. The
coordinator retains a clean detached `4a75d479` checkout for trial provenance.
Build and browser jobs have drained. These correctness results make no performance claim.

## Measured delivery

The coordinator completes all six declared browser runs before transferring report and PR ownership.
Read [the measurement report](74-benchmark-results.md) and [exact-value summary](74-benchmark-results.json) before follow-up work.
The audit verifies 120 starts/reaps, maximum one live worker, 2,400 samples, and 60 exact role pairs.
All 15 callback-overhead cases pass. Callback-disabled comparison passes on Firefox and WebKit.
Chromium cold Lab76 and Lanczos3 remain inconclusive. No new confirmed regression, retry, or tuning claim follows.
Inherited S32 cold regressions remain S41 release blockers; the coordinator owns their carry-forward.

The measured runtime remains `4a75d479d38a92c75e8ff4ed96c916fec3aaf8f4`.
The merge-preserving rebase onto final S32 `127a0428` preserves delivery head `06d9ad07` without tree changes.
Report-only commits follow it. Keep the detached measured-source worktree and all prepared artifacts/evidence.
After filing the real unmerged PR, return only the six listed S33 compiler targets to the coordinator for cleanup.
