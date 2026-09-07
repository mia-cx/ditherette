# Restore landed production code

Mia explicitly directs restoration of the landed optimized resize kernels and shared helpers.
The v1 plan covers remaining work. It does not authorize replacing completed implementations with naive copies.

## TODOs

- [x] Correct current execution guidance and visible plan copies to preserve landed production and reuse shared code.
- [x] Restore area/bilinear and cubic/Lanczos/convolution paths and callers in their existing slice worktrees, verifying original bytes.
- [ ] Restore nearest and its shared helpers, then connect the new public wrapper through required bounded/fallible integration.
- [ ] Verify restored content, original correctness checks, public ownership/errors, Wasm builds, and the trusted frozen-reference guard.
- [ ] Publish corrective commits and an unmerged restoration PR; reconcile current slice branches and progress.

## Ownership and constraints

The S21 agent restores area/bilinear in its existing isolated worktree. The S22 agent restores cubic/Lanczos/convolution separately.
The nearest agent uses a new isolated restoration worktree and leaves the interrupted diagnostic tree intact.
The coordinator owns this integration branch, plan/issue corrections, and PR filing.
Existing helper implementations remain production and should support new kernels wherever semantics fit.
New memory/ABI integration must preserve the public contract without replacing optimized kernel arithmetic.
The frozen spec, shared image checkpoint, and policy stay unchanged. Keep historical commits and evidence recoverable.
No benchmark, performance claim, merge, publication, tag, or deployment belongs to this restoration.

## Inspected state

S20 delivery `e19e12c219605138399cabd71b84cd4d9262a678` replaces nearest, while other landed resize kernels remain at their original paths.
The unmerged S21 and S22 working branches additionally replace area/bilinear and cubic/Lanczos/convolution.
All twelve area/bilinear implementation files remain byte-identical under candidate names.
The ten convolution-family files have only path imports and candidate-status comments changed; their kernel arithmetic is intact.
Eleven inherited resize tests pass against S20. Some check exact equality; fractional area and bilinear retain their bounded comparisons.
These test results establish existing fixture behavior, not universal exactness or performance.

## Restored area and convolution families

S21 restoration is `23f6e4f5b9bb6cc1110322b83b8538ffd6dd4508`.
S22 restoration is `55b08b4ad4911fc8aa3d65a86a9b94c801079962`.
The coordinator independently checks each with `git diff --exit-code 711c7aec RESTORATION_SHA -- crates`; both have zero differences.
Original kernels, shared plans/helpers, callers, module declarations, benchmark identities/profiles, and original tests are restored.
Each agent verifies four original tests, native/Wasm compilation, formatting, and the separately trusted frozen-reference guard.
Unused replacement kernels, copied helpers, and replacement-only tests are removed. Git history retains them and their copy manifests.
Both corrected branches join this restoration branch without source conflicts. Their final source trees add no crate changes to S20.
