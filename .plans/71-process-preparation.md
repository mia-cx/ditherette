# S30 end-to-end process preparation

Issue #71 completes the fifth public method. The prerequisite join is ready;
process implementation and its benchmark handoff remain separate work.

## Start condition and authority

Read the [approved PRD](../docs/plans/ditherette-v1/README.md) and
[S30 contract](../docs/plans/ditherette-v1/slices.md#s30). Read the resolutions
and addenda on issues #35, #40, and #37. The frozen composition is
`crates/ditherette-wasm/src/spec/pipeline/mod.rs::process_with_progress`.

Before editing runtime code, join the validated S23, S22, S25, S27, S28, and S29
heads in a clean S30 worktree. Record their exact SHAs and selected candidates.
S28 retains the selected ring. S29 retains its literal implementation because
the converter candidate's gates were inconclusive. Keep the rejected S25
dispatch and S26 converter out of the production call graph.

The isolated worktree is `.worktrees/v1-s30-process`. Its base branch is
`impl/v1-s30-base`; implementation follows on `impl/v1-s30-process`.
The coordinator start is `246297c94dda86b33b678d11eb194530c0b513a8`.

| Required slice | Delivered prerequisite |
|---|---|
| S23 | `cd7a0d298755818f86d710bef9f094c815d127de` |
| S22 | `9eecc670d9ff587ff10f8d2f3a8b86bab600c988` |
| S25 | `af8259ac766268e78690a569c10494c62cdb7ce2` |
| S27 | `c9666288cbe03a9f4dcfb14042cfcbff0fe61ca7` |
| S28 | `f4dfef7401d5474ac7318302d117ee0345449793` |
| S29 | `6eb9e00fd3191fc8bbd03559e89c67c762abfc25` |

The join preserves both dispatch paths, family tags 0 through 3, diffusion
error tags 26 through 35, and Yliluoma size tag 36. Shared placement parsing
retains S28's named paths. `preparation_failure` retains sibling visibility
for diffusion. No kernel, frozen spec/image, color, palette, resize, or freeze
policy changes belong to this join.

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

- [x] Materialize and validate the prerequisite join. Check each retained resize,
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
  gates. The coordinator owns the slice table, ledger, and cache cleanup.

## Join validation

Fresh scalar and threads builds pass through `pnpm --filter ditherette build`.
The installed dependencies use an ordinary offline pnpm installation. Generated
Wasm comes from this joined source, not the coordinator's old staged artifacts.
Native calls use the assigned S24 quantize target explicitly. Only the local
ignored target's scalar/threads children link to the assigned variant caches.

Focused checks pass with 38 native tests, 31 public interface tests, and 14
private-boundary tests. Native resize tests retain their inherited frozen
differences. Byte comparisons preserve the delivered diffusion ring and literal
Yliluoma modules. The only quantize runner difference from S29 is the sibling
visibility needed by S28. Browser installation checks follow with the complete
process implementation; no measurements run during this join.

The public recipe follows the frozen version-one shape, including its serialized
`match` key. Processing errors need recipe-relative paths where applicable.
Check caller getters and reentry before raw settings access, as current methods do.

## Scope limits

No rewrite of existing production kernels or shared helpers. Frozen spec/image
and freeze policy remain unchanged. No caches, progress delivery, threads,
website changes, UI work, publication, deployment, or PR merging in this slice.
S31 onward owns the later runtime work. Supplied progress callbacks retain the
current explicit unsupported behavior until S33.
