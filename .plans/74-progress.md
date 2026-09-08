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
5. [ ] Join installed fixtures and the assigned overhead protocol; validate fresh artifacts.
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
