# Ditherette v1 implementation stack

Read the [execution contract](README.md#execution-contract) before claiming a slice. Each entry records the validated dependency commits and outstanding evidence.

## Current correction: retain landed production

Mia directs restoration of the landed optimized kernels and shared helpers in [#108](https://github.com/mia-cx/ditherette/issues/108).
This supersedes earlier copied-baseline execution instructions for already-landed implementations, not the frozen reference itself.
The coordinator owns `fix/v1-restore-landed` in `.worktrees/v1-restore-landed`, based on S20 `e19e12c219605138399cabd71b84cd4d9262a678`.
S21 restores area/bilinear; S22 restores cubic/Lanczos/convolution. Both restored heads join this correction.
Nearest restoration `d17e323d2ba6487474c6fa9952e03a86d393a8ba` reconnects the public API with bounded/fallible integration.
The correction is validated in open, non-draft [PR #109](https://github.com/mia-cx/ditherette/pull/109), targeting `impl/v1-s20-browser-bench`.
S19 is ready with this correction included. Existing PRs and historical evidence remain open and recoverable.
S20 tooling evidence remains valid for the artifact it measured; it does not establish performance of the restored implementation.
The interrupted diagnostic branch stays separate and will not delay restoring landed code.
All later work reuses landed kernels and shared helpers. Only missing implementations start from literal spec copies.

## Current implementation

Current integration work continues on `impl/v1-resize-integration` in `.worktrees/v1-resize-integration`.
It owns the tracked progress table; the root table remains the visible mirror.
S21 public area/bilinear checkpoint `056a1324` joins at `b52d1c8b`, with native/package/three-engine conformance passing.
S22 public convolution checkpoint `f0976601` preserves native checkpoint `53eaf013` and passes all three package engines.
S23 public trilinear is validated at `0be73eb6`, retaining literal missing-implementation baseline `fba85a94` and prepared checkpoint `830e6739`.
It passes 299 native, 18 interface, eight private ABI, both builds, and all three installed-package engines.
S24 native packed-color/direct quantization is validated at `f83e58a4`; public integration and benchmark registration continue separately.
The coordinator owns benchmark protocol/adapters, joins, and exclusive measurements.
A separate owner adds native budgeted subjects in `impl/v1-resize-bench-subjects`; no production files belong to that task.
S21/S22 measurements complete all 304 serial workers and retain 5,760 samples. No measurement is running.
All native production pairs preserve landed bytes, with no slowdown above 10%; no kernel retuning is selected.
The [S22 measurement record](../../../.plans/63-measurement.md) links artifacts and records each convolution median.
Public TypeScript differences and complete-call costs remain [S41 work](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5575617483).

## Delivered S01 through S16

This snapshot comes from the live PR state after S16 delivery. All 16 PRs are open, non-draft, and have auto-merge disabled.
The linked PR descriptions contain each slice's validation evidence. Worktrees have the branch suffix under `.worktrees/`, without `impl/`.
Use [the test-count audit](test-count-audit.md) for native totals. Original issue prerequisites remain the dependency authority.

| Slice / issue | PR | Branch | Immediate PR base | Delivered head |
| --- | --- | --- | --- | --- |
| S01 / [#42](https://github.com/mia-cx/ditherette/issues/42) | [#75](https://github.com/mia-cx/ditherette/pull/75) | `impl/v1-s01-anchor` | `main` | `a213effed4b426c5c432c9ccc7062b7016dd5c1b` |
| S02 / [#43](https://github.com/mia-cx/ditherette/issues/43) | [#88](https://github.com/mia-cx/ditherette/pull/88) | `impl/v1-s02-builds` | `impl/v1-s01-anchor` | `bc110d91d441ec3069d128e3cc7d39b3b439a1f4` |
| S03 / [#44](https://github.com/mia-cx/ditherette/issues/44) | [#89](https://github.com/mia-cx/ditherette/pull/89) | `impl/v1-s03-contracts` | `impl/v1-s01-anchor` | `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06` |
| S04 / [#45](https://github.com/mia-cx/ditherette/issues/45) | [#90](https://github.com/mia-cx/ditherette/pull/90) | `impl/v1-s04-bench-lock` | `impl/v1-s01-anchor` | `b5ed4d250cbdc804f18bbadb24df4463db1b6434` |
| S05 / [#46](https://github.com/mia-cx/ditherette/issues/46) | [#96](https://github.com/mia-cx/ditherette/pull/96) | `impl/v1-s05-verification` | `impl/v1-s05-base` | `8c05906cb9cfe0a991b351f5260a319b91f76ab4` |
| S06 / [#47](https://github.com/mia-cx/ditherette/issues/47) | [#102](https://github.com/mia-cx/ditherette/pull/102) | `impl/v1-s06-paired-bench` | `impl/v1-s05-verification` | `a65f53e24b880932021b39e601b5682b1111bc54` |
| S07 / [#48](https://github.com/mia-cx/ditherette/issues/48) | [#91](https://github.com/mia-cx/ditherette/pull/91) | `impl/v1-s07-color-basic` | `impl/v1-s03-contracts` | `7ef52bd2bcaea2774a400875e5c395526bc9b4b9` |
| S08 / [#49](https://github.com/mia-cx/ditherette/issues/49) | [#92](https://github.com/mia-cx/ditherette/pull/92) | `impl/v1-s08-color-perceptual` | `impl/v1-s03-contracts` | `d5e2d9761481f7a6b74fb37c7c2f7841570bead1` |
| S09 / [#50](https://github.com/mia-cx/ditherette/issues/50) | [#94](https://github.com/mia-cx/ditherette/pull/94) | `impl/v1-s09-palette` | `impl/v1-s03-contracts` | `d8bcdcdbe8f874eaee65447a640b97483c9cb775` |
| S10 / [#51](https://github.com/mia-cx/ditherette/issues/51) | [#97](https://github.com/mia-cx/ditherette/pull/97) | `impl/v1-s10-quantize` | `impl/v1-s10-base` | `47712a500c4079293a06fae4a07ae105a643af8f` |
| S11 / [#52](https://github.com/mia-cx/ditherette/issues/52) | [#93](https://github.com/mia-cx/ditherette/pull/93) | `impl/v1-s11-resize` | `impl/v1-s03-contracts` | `bb9421cbecdeadb3c70c78d1e7412f982be7053f` |
| S12 / [#53](https://github.com/mia-cx/ditherette/issues/53) | [#95](https://github.com/mia-cx/ditherette/pull/95) | `impl/v1-s12-placement` | `impl/v1-s12-base` | `01df66826e532d8fb3b522a1564f1121c96f4d1f` |
| S13 / [#54](https://github.com/mia-cx/ditherette/issues/54) | [#98](https://github.com/mia-cx/ditherette/pull/98) | `impl/v1-s13-perturb` | `impl/v1-s13-base` | `2a2b0f2de2592cba424c5823f40e3604f1b2a6b7` |
| S14 / [#55](https://github.com/mia-cx/ditherette/issues/55) | [#99](https://github.com/mia-cx/ditherette/pull/99) | `impl/v1-s14-blue-noise` | `impl/v1-s13-base` | `769050190114ca584669703be2be0042c035aa9a` |
| S15 / [#56](https://github.com/mia-cx/ditherette/issues/56) | [#101](https://github.com/mia-cx/ditherette/pull/101) | `impl/v1-s15-diffusion` | `impl/v1-s15-base` | `cfec5d9b7ab9c1d7c81c1e6a40f7476f75be9788` |
| S16 / [#57](https://github.com/mia-cx/ditherette/issues/57) | [#100](https://github.com/mia-cx/ditherette/pull/100) | `impl/v1-s16-yliluoma` | `impl/v1-s15-base` | `d58355e617bf17fa3481f0f165dd4646e856ce7f` |

### S06 measurement evidence

The tooling PR is ready; its measured control candidate is not accepted production.
The first exclusive native trial ran four alternating accepted/candidate pairs for latency and throughput.
All 16 children exited, each reported one live benchmark, and 1,600 raw samples remain in the artifacts.
Every output proof was exact and pre-freeze. Both host locks were free after the run.

Accepted control `cf132cdd44bb3ba13974544d1c683af92bd2fd22` and candidate control `87cff9aaab4a3bb5d033645d21bf1ed615a98455` use unchanged reference nearest code.
The observed difference is not diagnosed as a source-level optimization effect.
Throughput regressed 10.4795% across all four pairs. Latency was 10.1819% slower, but its pair ratios crossed the gate, so it is inconclusive.
The coordinator exited 2. No candidate promotion, retry, or browser-performance claim followed.

Raw artifacts are at `.worktrees/v1-s06-paired-bench/crates/ditherette-bench/target/s06-controls/trial-01/`.
The report digest is `44f9464134438271fc0875d1589b70822c3cfa0814fd3ea8daec36397797ee76`.
The event journal digest is `fcd7897e12708cf646942649e7941a63b02ca68d18187b80d0113e7b30e78ef4`.
PR #102 retains the tool protocol, identities, checks, and raw-artifact manifest.

## S17 reference integration

The coordinator owns `impl/v1-s17-processor` in `.worktrees/v1-s17-processor`.
Its immediate PR base is `impl/v1-s17-base` at `4f4b48a04c0ccee51222b7e621ff4a566a495613`.
This join contains the delivered S05, S10, S11, S13, S14, S15, and S16 heads above, including their own dependencies.
S02 and S06 remain separate prerequisite branches for S19; they are not silently omitted from the overall delivery.

The base passes 226 actual listed native tests, Wasm compilation, and formatting.
Its explicit integration correction connects S13 field dispatch to S14's corrected blue-noise lookup.
All satisfied native blockers were removed from issue #58 after the validated join existed.
The original issue dependency list remains unchanged.

[PR #103](https://github.com/mia-cx/ditherette/pull/103) is open and non-draft, with auto-merge disabled.
Its creation head is `0fb89b929108b68eb3486e1c6b9834d9351e1a4f`; validated code checkpoint is `4ffbad8f1ae09bc81f316444cd27433a436c35ab`.
Subsequent handoff commits change documentation only. S18 must record its actual selected parent head before freezing.

All 270 native tests, Wasm compilation with benchmark subjects, and crate formatting checks pass.
The benchmark crate passes 5 reference-subject, 8 verification, 3 adapter, and 3 binary tests, plus the lease lifecycle fixture.
That fixture runs the three owned Node transport checks under the required inherited lease.
Native benchmark binaries and benches compile. No S17 measurements ran.

The required rebase preserved the complete tree and all seven slice prerequisite heads in ancestry.
Subtask commits were replayed; [the S17 task plan](../../../.plans/58-reference-processor.md) records their source provenance and exact post-rebase checks.
The coordinator alone updates the visible root `slices.md` and the current tracked integration copy.


## S18 reference freeze

Issue [#59](https://github.com/mia-cx/ditherette/issues/59) runs in `.worktrees/v1-s18-freeze` on `impl/v1-s18-freeze`.
Its immediate parent is `impl/v1-s17-processor` at `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
The prerequisite commit is in ancestry. Its native blocking edge was removed only after this worktree existed.
Validated implementation and creation head is `e636b3120f566127b5e6b884ff2df3cd24c7c5ca` in [PR #104](https://github.com/mia-cx/ditherette/pull/104).
The PR is open and non-draft, with auto-merge disabled.
CI found a cold-cache fixture setup failure. Fix `c03c3c85f5748b7726cfdae50ea6d41acd8ede64` passes [run 34131713248](https://github.com/mia-cx/ditherette/actions/runs/34131713248), including all 11 mutation fixtures and the guard.
Its follow-up delivery is `662d6483`; the frozen digest and guard semantics are unchanged.
The fixed checkpoint remains the S17 parent, retained by branch `reference/ditherette-v1`.
The guard uses recorded bytes, never that branch's tip or a new parent.
All 106 reference/shared-image/provenance files match SHA-256 `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Its full export inventory and blue-noise generator, asset, and analysis belong to this frozen closure.

The complete guard passes compiler/profile checks, four dependency contexts, native/Wasm isolation, and native threaded production isolation.
All 11 temporary mutation tests pass and restore their temporary trees.
Independent review corrections cover foreign/linker symbols, procedural expansion, and raw-identifier bypasses.
Rust formatting, Prettier, and diff checks pass. No benchmark ran.
The read-only CI workflow executes exact-base policy. Its first S18 bootstrap requires review because no base guard exists yet.
Repository-required checks and workflow protections are not activated by this unmerged preparation.
The coordinator owns progress, aggregate integration, and PR filing.

The separate S19 preparation branch `impl/v1-s19-base` now joins S02, S06, and S17 at `ad2410b481d710fb2d739695ec1637d6d08eab79`.
All three delivered prerequisite SHAs are ancestors; the merges needed no source conflict resolution.
The joined tree passes native Rust tests, Wasm compilation, formatting, and the combined benchmark verifier/paired/lease fixtures.
Its `spec/`, `image/`, and both consumer lockfiles exactly match the S17 parent.
The reviewed S18 head `eee0b5ddfb600b9ba6517c3dcb755e3566fc7813` now joins this base.
Its trusted guard passes the complete resolved tree at `363324c43556f08ef4e8677d226f5659977bcc66`.
Documentation-only join `1f7e7a68803f5af6953ec710c536cbb66a32c1f8` retains identical crates, guard, and workflow content.
S19 started in `impl/v1-s19-scalar` for the literal Rust baseline and `impl/v1-s19-package` for private glue generation.
The coordinator owns progress and joins. Neither child runs benchmarks during implementation.
The corrected S18 fixture joins this base at `1dd8128a8532638ee2a17853e562145bad687e3f`.
The separate trusted S18 guard passes that joined tree.
This prerequisite branch contains no new production implementation.

## S19 scalar package implementation

Issue [#60](https://github.com/mia-cx/ditherette/issues/60) is implemented in open, non-draft [PR #105](https://github.com/mia-cx/ditherette/pull/105), with auto-merge disabled.
The final slice joins in `.worktrees/v1-s19-integration` on `impl/v1-s19-integration`, based on `impl/v1-s19-base`.
This worktree owns the current tracked progress and ledger; the root `slices.md` remains the visible mirror.

- Literal production baseline `0ede7f6c6f90d6c5d40b169b1dd835f0ac752902` has five verified copies and a separate unchanged legacy candidate.
- Policy join `89b570e0dbb4280352b157bfde5b20c3a7e80a9e` retains the exact baseline and passes the separately trusted S18 guard.
- Private factory delivery `8ecdf786191a451ad13eb0f65191341101ffdc01` passes seven focused fixtures, actual scalar/threaded builds, declaration checks, and inert-import validation.
- The coordinator joined both deliveries without conflicts. Private Rust allocation handling and the public TypeScript wrapper continue in their assigned worktrees.
- Native candidate work runs in `impl/v1-s19-nearest-opt`; the accepted binary builds from clean `89b570e0` in `impl/v1-s19-nearest-accepted`.

The baseline has 280 passing native tests. It is not a completed public package call.
The first S19 experiment now passes all ten native cases with exact output, 8,000 samples, and 80 sequential children reaped.
Accepted artifact `89b570e0` and candidate artifact `f9b51e45` produce about 52% lower resize latency and 96.6% lower identity latency.
The [measurement report](../../../.plans/60-nearest-measurement.md) records exact revisions, executable hashes, raw artifact locations, and every median.
All three agents and builds stop for the measurement; every child exits before implementation resumes.
Promotion `964683f46a24c248aa91318bd6280ca91bec88f7` preserves tested arithmetic and passes 285 native tests, Wasm compilation, and the trusted guard.
Delivery `b48511321f47722fcfb48c51a40a05a825f06a7b` joins at `fdef72b5788c3c09fc6439991a01bfc851dc71ce`.
The final code join is `a9835a97276bfc726590f931caa2af1b4b0b3d9a`, including the private Rust adapter, public wrapper, and corrected tarball staging.
It passes both Wasm builds, 285 native tests, 12 public/validation fixtures, public type checks, two staging fixtures, five factory fixtures, and six private Wasm fixtures.
The installed tarball passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 using a task-local WebKit library launcher.
Trusted freeze enforcement and Rust formatting pass. A merge-preserving rebase keeps the exact code head and all required checkpoint ancestry unchanged.
S20 still owns complete browser-call timing; native evidence and browser conformance do not replace it.
Documentation/provenance joins at `db2dbb61c9d6d14723c3f3bfb05a0d3465d6e057`; the crate and package trees match the validated code head.
The PR's current head is authoritative for subsequent progress-only commits. The issue remains open until a separately authorized merge.

## S20 public browser benchmark implementation

Issue [#61](https://github.com/mia-cx/ditherette/issues/61) starts from validated S19 PR #105 at `7de86d799a25a132c8de41ee54696bd8e54bdf76`.
The immediate PR base is `impl/v1-s19-integration`. The coordinator owns `.worktrees/v1-s20-browser-bench` on `impl/v1-s20-browser-bench`.
This worktree now owns tracked progress and the ledger; root `slices.md` remains the visible mirror.
Read [the scoped plan](../../../.plans/61-browser-benchmark.md) and [preflight](../../../.plans/61-browser-preflight.md) before implementation.
Protocol, script transport, and asset/worker implementation use separate child worktrees. Measurements wait until every agent and build/test exits.
Trial 02 completes with exact output but performance regressions in every engine. No release-performance readiness claim exists.
S19's exact delivered head passes CI run `34136269326`; its frozen-reference check and PR status are green.
The [initial trial budget](../../../.plans/61-initial-trial-budget.md) fixes the case matrix before measurement.
The [measurement report](../../../.plans/61-public-measurement.md) records actual medians, runtime identities, retained failures, and S41 obligations.
Both package roles build from clean `e84a55eddb0014f97b64446408bfb5f656deb5d4`; all three engine coordinators complete with regression exit 2.
No performance candidate is promoted. A developer-only same-kernel comparison investigates the gap from older internal Wasm measurements.
S20 is delivered in open, non-draft [PR #107](https://github.com/mia-cx/ditherette/pull/107), with creation head `b97e0e5b20b198692fe37944f89ccdfc51734426`.
Its immediate base remains `impl/v1-s19-integration` at `7de86d799a25a132c8de41ee54696bd8e54bdf76`.
All 45 Rust benchmark tests and 20 focused JavaScript tests pass after the unchanged rebase.
The independent audit verifies every sample-derived gate, 216 exact outputs, and complete snapshot identities.
Ready describes the tooling PR, not release performance. S41 retains the confirmed regressions.

## S21 and S22 remaining integration

Both branches restore their crate trees to S20 delivery `711c7aec61587b45a91c2e404583161edb1e0de9`.
S21 restoration is `23f6e4f5b9bb6cc1110322b83b8538ffd6dd4508`; S22 restoration is `55b08b4ad4911fc8aa3d65a86a9b94c801079962`.
The coordinator verifies both source trees byte-for-byte and joins them into `fix/v1-restore-landed`.
Both clean branches fast-forward to validated correction `c3e00ffee699d655f0c9fd5cfa56e25b7f1ef3e3`.
S21 targets `fix/v1-restore-landed`. S22 stacks on S21 to reuse its validated public integration.
S21 is open in [PR #110](https://github.com/mia-cx/ditherette/pull/110) at `7743c2e5b4a313fa5e7da70b850956b85a880df4`.
S22 is open in [PR #111](https://github.com/mia-cx/ditherette/pull/111) at `2c2c1994b40eb65246b9f434322f1c20b7f260bf`.
Both are non-draft and unmerged, with auto-merge disabled. Neither grants release-performance readiness.
S22's merge-preserving rebase retains original checkpoints and measured runtime bytes.
Its final branch passes 296 native, 16 interface, seven private ABI, both builds, and three installed-package engines.

| Slice | Branch/worktree suffix | Current task |
| --- | --- | --- |
| S21 / #62 | `v1-s21-area-bilinear` | Ready in PR110 |
| S22 / #63 | `v1-s22-convolution` | Ready in PR111, stacked on PR110 |

Branches have the `impl/` prefix; worktrees live under `.worktrees/`.
Existing optimized kernels and shared helpers stay canonical production. Removed replacements remain recoverable in historical commits.
Public processor/package integration and fresh measurements are complete for S21/S22. No new optimization is selected.
The interrupted nearest diagnostic in `impl/v1-s20-nearest-diagnostic` stays held and separate.

## S01 inherited port anchor

- Issue: [#42](https://github.com/mia-cx/ditherette/issues/42), parent PRD [#41](https://github.com/mia-cx/ditherette/issues/41).
- Owner: S01 anchor agent. Worktree: `.worktrees/v1-s01-anchor`.
- Branch: `impl/v1-s01-anchor`. PR base: `main`. PR: [#75](https://github.com/mia-cx/ditherette/pull/75), open and non-draft.
- Main: `edc87f5da2b6958f7d9c892483e08af8149482b4`.
- Inherited port: `5a5872badbba796f4effe92aba8051b43e30233e` (`origin/feat/rust-wasm-port`).
- Rebased code checkpoint: `a9928ebe55a2571b4e6bb35fedea80aff5474302`. Dependencies: none.
- Validated code and plan head: `0f33dbdf123cf367362a8473c2e95bf1223a2c3c`. Later S01 commits record evidence only; the PR records the delivered head.

All 448 inherited commits were replayed onto main. Two `.gitignore` conflicts preserved the port's fixture changes and main's `.ant-colony/` exclusion. The rebased tree differs from the inherited port only by that exclusion. Original port history remains on its existing branch. Root worktree edits remain untouched.

### Baseline evidence

Checks run in the isolated worktree at the rebased code checkpoint:

| Command | Result |
| --- | --- |
| `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked` | Passed, 100 native tests; no doctests. |
| `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown` | Passed. |
| `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` | Passed. |
| `git diff --check origin/main...HEAD` | Passed. |
| `pnpm install --frozen-lockfile` | Existing setup failure under pnpm 11.13.0: `ERR_PNPM_IGNORED_BUILDS`. The inherited `onlyBuiltDependencies` configuration does not satisfy pnpm 11's `allowBuilds` policy. Dependencies were installed; generated manifest placeholders were removed. |
| `pnpm exec vitest run --project server src/lib/processing src/lib/wasm/ditherette-wasm.spec.ts` | Blocked by pnpm's automatic reinstall and the same build-policy failure. |
| `./node_modules/.bin/vitest run --project server src/lib/processing src/lib/wasm/ditherette-wasm.spec.ts` | Passed, 16 files and 120 tests. |

Tool versions: Rust/Cargo 1.97.0, Node 24.19.0, pnpm 11.13.0.

No new baseline failures were found. S02 owns the inherited package/build policy issue. Direct Vitest invocation validates processing and wrapper tests without changing that policy.

No benchmark process ran. Native export tests include their existing benchmark-wrapper smoke assertions; these do not establish performance evidence. Browser, threaded-runtime, full website-build, and release conformance checks remain later slice obligations.

### Next dependencies

S01 implementation is available at validated code/plan head `0f33dbdf123cf367362a8473c2e95bf1223a2c3c`, with evidence at `60bfa968e9541f14e2e1490a7829f91dda490d89`. This subsequent entry only records the PR URL. S02, S03, and S04 can use the delivered PR head, which contains both checkpoints. Child PRs target `impl/v1-s01-anchor` and record its exact SHA. All PRs remain unmerged.
