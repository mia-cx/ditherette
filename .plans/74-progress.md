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

1. [ ] Add the allocation-free callback boundary and copied-model progress controller.
   Compare fake-clock schedules and completion permission with frozen lifecycle fixtures.
2. [ ] Add countable, fallible hooks to existing kernel loops without changing their outputs or allocations.
   Prove callback-disabled equality and abort behavior with focused native tests.
3. [ ] Connect all five pipelines and success-only completion, including image hits.
   Prove no new publication after intermediate/completion failure and successful recovery.
4. [ ] Enable typed request callbacks through the caught Wasm boundary.
   Verify callback errors, getters/reentry/disposal, result readiness, and private handle recovery.
5. [ ] Join installed fixtures and the assigned overhead protocol; validate fresh artifacts.
   Record exact source/artifact identities, await exclusive measurements, and file an unmerged child PR.

Each completed step is a buildable commit. Count callback/control records in the
existing ownership budget; callback-disabled calls must avoid clock imports.
