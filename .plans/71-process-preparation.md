# S30 end-to-end process preparation

Issue #71 completes the fifth public method. This is a preparation note, not a
claim that its prerequisite measurements or implementation have finished.

## Start condition and authority

Read the [approved PRD](../docs/plans/ditherette-v1/README.md) and
[S30 contract](../docs/plans/ditherette-v1/slices.md#s30). Read the resolutions
and addenda on issues #35, #40, and #37. The frozen composition is
`crates/ditherette-wasm/src/spec/pipeline/mod.rs::process_with_progress`.

Before editing runtime code, join the validated S23, S22, S25, S27, S28, and S29
heads in a clean S30 worktree. Record their exact SHAs and selected candidates.
S28's ring and S29's converter candidate remain unselected while measurements
are pending. Do not import the rejected S25 dispatch or unselected S26 converter.

## Existing implementation to reuse

- `prod/pipeline/resize.rs::PreparedResize` already owns landed resize plans,
  scratch, capacity accounting, and dispatch, including shared-mip trilinear.
- `prod/pipeline/processor.rs` owns per-instance lifecycle and caught input/output
  boundaries. `prod/pipeline/quantize.rs` owns prepared palette/matching calls.
- The S28/S29 join supplies bounded diffusion and Yliluoma dispatch. Reconcile
  their shared registration changes; preserve their algorithm implementations.
- Package validation, numeric private error paths, and caught void result sinks
  already exist. Extend these conventions for the process request.

Read the joined files again before editing. This inventory reflects the S26
coordinator and reviewed S28/S29 code, not a substitute for the actual join.

## Atomic work

- [ ] Materialize and validate the prerequisite join. Check each retained resize,
  color, palette, and dither implementation against its delivered source tree.
- [ ] Add only the missing process composition, following the readable frozen
  resize-then-dither sequence. Record its baseline before optimizing ownership.
  Keep intermediates in Wasm. Only the final indexed result crosses to JS.
  Preserve the RGBA8 clipping and rounding boundaries between stages.
- [ ] Register the versioned public process request and private export. Reuse
  existing option types and validation. Reject the entire invalid request before
  output work, and count simultaneous prepared data, source, intermediates,
  indices, scratch, and boundary capacity. Test allocation failures and recovery.
- [ ] Verify process equals the actual staged production calls for each family,
  including indices, ordered palette, transparency, warnings, and durable output.
  Retain independent frozen comparisons and any inherited resize differences.
  Do not change a landed kernel or the frozen oracle to erase those differences.
- [ ] Add complete-call native/public benchmark adapters and extend the existing
  identified frozen-Wasm oracle with the frozen process composition. Declare a
  bounded, filter-specific trial before changing the baseline. Measure serially
  only after all agents, builds, and tests drain. Keep exact wins only.
- [ ] Run focused integrated native, private-boundary, installed-tarball, and
  three-engine checks. Open an unmerged PR against the documented parent/join.
  Record dependency ancestry, artifact identities, measurements, and outstanding
  gates; update the slice table and ledger.

The public recipe follows the frozen version-one shape, including its serialized
`match` key. Processing errors need recipe-relative paths where applicable.
Check caller getters and reentry before raw settings access, as current methods do.

## Scope limits

No rewrite of existing production kernels or shared helpers. Frozen spec/image
and freeze policy remain unchanged. No caches, progress delivery, threads,
website changes, UI work, publication, deployment, or PR merging in this slice.
S31 onward owns the later runtime work. Supplied progress callbacks retain the
current explicit unsupported behavior until S33.
