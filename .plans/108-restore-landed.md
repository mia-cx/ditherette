# Restore landed production code

Mia explicitly directs restoration of the landed optimized resize kernels and shared helpers.
The v1 plan covers remaining work. It does not authorize replacing completed implementations with naive copies.

## TODOs

- [x] Correct current execution guidance and visible plan copies to preserve landed production and reuse shared code.
- [x] Restore area/bilinear and cubic/Lanczos/convolution paths and callers in their existing slice worktrees, verifying original bytes.
- [x] Restore nearest and its shared helpers, then connect the new public wrapper through required bounded/fallible integration.
- [x] Verify restored content, original correctness checks, public ownership/errors, Wasm builds, and the trusted frozen-reference guard.
- [x] Publish corrective commits and an unmerged restoration PR; reconcile current slice branches and progress.

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

## Nearest restoration and final validation

Exact restoration commit `1f8c8d903642baf7437b87c8a42c292dbece54d8` restores all five original nearest files.
Boundary commit `2476b4cc` adds fallible, capacity-accounted planning while retaining landed pixel execution.
Final implementation head `d17e323d2ba6487474c6fa9952e03a86d393a8ba` also restores canonical row-band benchmark dispatch.
The public processor uses the existing planned packed kernel and its identity bypass.
Original nearest alignment/packed files, the common helper tree, and all other restored resize families match `2b4bfa8e` byte-for-byte.
Only nearest allocation integration changes the original plan/module/span helpers. Mapping and span arithmetic remain unchanged.
The replacement algorithm, unused copied alignment helper, and replacement-only tests are removed; Git retains their history.

The coordinator rebuilds and tests final head `d17e323d` in its prepared worktree.
The integrated correction matches that head exactly under `crates/`, `packages/`, and `scripts/`, including after the merge-preserving rebase.

| Check | Result |
| --- | --- |
| Full native crate tests with bench-subjects | 280 pass, including original resize fixtures and allocation recovery |
| Scalar and threaded release package builds | Both pass |
| Public interface/types and Node fixtures | 12 pass |
| Actual private Wasm fixtures | 6 pass |
| Installed tarball browser contract | Chromium 147.0.7727.15, Firefox 148.0.2, WebKit 26.4 pass |
| Row-band registration fixture | Pass, reported by implementation agent |
| Rust formatting and diff checks | Pass |
| Independently trusted freeze guard on integrated correction | Pass |

The frozen revision remains `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
Its digest remains `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
A read-only independent review finds no concrete allocation, cleanup, mapping, or dispatch defect.
No benchmark runs and no restored-performance claim follows from these correctness checks.
S21/S22 still need public package integration and their remaining slice evidence.

## Delivery

Open, non-draft [PR #109](https://github.com/mia-cx/ditherette/pull/109) targets `impl/v1-s20-browser-bench`; auto-merge is disabled.
Its creation head is `c3e00ffee699d655f0c9fd5cfa56e25b7f1ef3e3`.
Both clean S21/S22 branches fast-forward to that validated correction; their next PRs target `fix/v1-restore-landed`.
The progress table links the correction beside S19. S21/S22 remain in progress.
Root visible progress matches the tracked table. The user-owned workspace configuration remains untouched.
