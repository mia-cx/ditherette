# S33 progress reuse inventory

Issue #74 remains dependent on validated S32. These read-only findings prepare its handoff, not implementation availability.
Read the approved [execution contract](../docs/plans/ditherette-v1/README.md#execution-contract),
[slice](../docs/plans/ditherette-v1/slices.md#s33), and
[resolved progress decision](https://github.com/mia-cx/ditherette/issues/24#issuecomment-5562864637) before implementation.

## Existing contract and ownership

`spec/contract/lifecycle.rs` already defines eight stages, measurable counts, the 50 ms within-stage gate,
output readiness, callback failure/reentry, and successful-publication permission.
Its production mirror already exists under `prod/contract/lifecycle.rs`. Preserve the frozen model and verify implementation against it.

`packages/ditherette/src/types.ts` already includes `onProgress` on every request.
The validators explicitly reject supplied callbacks until S33. Enable this existing option without adding cache controls.
`scalar.ts` guards the complete operation, including validation, with `#active`. Preserve that guard for getters and callbacks.

`wasm/processor.rs::take_ready` moves the processor into `Slot::Busy` without retaining a mutable `RefCell` borrow across JavaScript.
The copy helpers use caught void imports and a private result sink. Extend this boundary without introducing owned-return handle leaks.
Numeric callback error status and its TS message already exist. The numeric error-path table still needs an aligned `onProgress` entry.

## Final output and publication

At S32 checkpoint `f1e4bdc7194f99ccb39fec4287dc3e72b7e8148d`, indexed methods share `prod/pipeline/indexed.rs`.
Resize and perturb retain their existing orchestration. Each path constructs its durable result through `boundary.complete`,
then calls `preparation::Call::finish`; drop publishes only successful pending entries.

Completion delivery must happen after durable output construction and before this success marker.
Delivering completion after Wasm returns is too late to prevent cache publication when a callback throws.
Keep early final-cache-hit paths under the same completion and failure rules.

## Countable work without replacing kernels

Inspect the actual S32 parent before editing. These paths exist in the delivered S31/S32 implementation:

- `prod/dither/perturb.rs::perturb_by_field_rows_into` accepts a global `RowBand` and preserves global random indices.
  It expects complete image dimensions, not an independently cropped row image. Adaptive neighbors use the full source.
- `prod/quantize/prepared.rs::quantize_into` and `prod/dither/yiluoma/request.rs::dither_yiluoma_into` already iterate output rows.
  Their existing per-pixel arithmetic and traversal must remain unchanged when reporting completed work.
- `prod/dither/error_diffusion/prepared.rs::execute_with_scratch` owns the continuous three-row feedback traversal.
  Progress cannot restart diffusion for each band or reset feedback at callback boundaries.
- `prod/pipeline/resize.rs::PreparedResize` dispatches the existing budgeted, caller-scratch kernels.
  Preserve this path and its exact arithmetic instead of selecting a different kernel just to report progress.
- `prod/resize/scalar/trilinear/prepared.rs` fills the shared mip chain, samples retained levels, and blends output rows.
  Counts must describe work actually completed, not imply all resize work finishes at the first mip boundary.

Existing legacy resize row functions are not automatically suitable for this bounded processor.
Area's `resize_rows_with_plan_into` allocates its own vertical row.
Convolution's `resize_x_then_y_rows_into` allocates and recomputes filtered source rows for each band.
Blindly switching to those functions can introduce unbudgeted memory and repeated work.
Use the existing bounded execution and shared helpers; benchmark callback-disabled/enabled paths before claiming acceptable overhead.

## Required evidence

Compare event behavior with the frozen lifecycle model. Verify all five public methods, cache hits,
thrown intermediate/completion callbacks, rejected reentry/disposal, recovery, and no new publication after failure.
Check ordinary output bytes and metadata remain unchanged. Stage changes report immediately; same-stage events respect 50 ms.
Prepare fresh artifacts before the approved focused overhead comparison. Measurements need coordinator quiet clearance.
Threads, website forwarding/cancellation, releases, and rollout activation stay in their later slices.
