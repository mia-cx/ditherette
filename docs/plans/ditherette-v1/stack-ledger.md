# Ditherette v1 implementation stack

Read the [execution contract](README.md#execution-contract) before claiming a slice. Each entry records the validated dependency commits and outstanding evidence.

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
The PR is open and non-draft, with auto-merge disabled. Later handoff commits change documentation only.
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
S19 remains unstarted and blocked on S18. This preparation contains no new production implementation.

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
