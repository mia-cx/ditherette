# Ditherette v1 implementation stack

## Stack collapse on 2026-09-12

S02 head `a53aef27` passes both pinned Wasm builds, package/version checks, 102 native tests, two Wasm overflow tests, 120 TypeScript tests and 15 deterministic benchmark-tooling tests. Fusion session `purple-twill` used Astra High and SWE-2 Medium. Finished targets were cleared, freeing 1.5 GiB.

Pullfrog finding `3997233677` blocks PR #88. npm marks pnpm 11.13.0 broken; pnpm's installer rejects its missing native binary. The installed JavaScript CLI passing locally does not validate fresh native installs. The active tip also pins 11.13.0 in package conformance and publishing workflows. Changing the shared pin to the non-deprecated 11.13.1 requires Mia's downstream-impact decision. No pin/source changes or PR skips. The thread remains unresolved and the watcher has exited.

PR #75 merged as `db1c3f6767a5e74a4abefdeaa36882ef807a0e45` after Pullfrog approved head `75de22dd`, CI passed, and all 18 threads were resolved. Issue #42 closed. Local main fast-forwarded. Children #88, #89, and #90 were retargeted before branch deletion. S02 now restacks onto main without conflicts and enters babysitting. About 2 GB of finished root build targets were cleared.

Mia authorizes root-first babysitting, merge, and restacking through active work, using Devin Fusion GPT-6 Astra High with SWE-2. Stop and flag actual conflicts, skipped PRs, or fixes that collide with downstream runtime/spec work.
PR75 has verified fixes `bda074ee` for output-storage overflow and `cd2a04dd` for invalid benchmark scales. The area, bilinear, and legacy website-only threads are resolved under the package-first scope; kernels and legacy adapter stay unchanged.
Generated fixtures and benchmark artifacts do not belong in the repository. Mia removes the legacy-artifact compatibility hold: no migration or old-baseline preservation gate is required for correct new-run fingerprints. This does not authorize bulk deletion of existing tracked files. Other review and merge gates remain active.

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

### Multi-setting JS performance work, 2026-09-11

[PR #148](https://github.com/mia-cx/ditherette/pull/148), issue #147, is open on PR #146.
Head `43fe220b` returns the original input for identity-only resize after validation and progress handling.
All 46 interface tests and Chromium, Firefox and WebKit identity-alias checks pass.
This is the user-approved ownership exception. Identity-only calls perform no Wasm call or pixel copy.

[Issue #149](https://github.com/mia-cx/ditherette/issues/149) tracks scalar performance against historical JS across scales and settings.
The 96-recipe Chromium screen is diagnostic, not a release gate. Three samples cannot qualify every cell.
Cold and warm public calls target 20% lower time than JS; end-to-end timing remains separately visible.
Zero-copy no-ops are resolution-limited, not claimed speedups. Many nontrivial cells still miss the target.

Integration branch `perf/v1-js-performance` at `f89b4444` adds converter reuse and finite metric specialization for diffusion.
All 16 compared Chromium output PNGs are byte-identical to the accepted Wasm package.
Adaptive sRGB warm time falls from 416.6 to 22.9 ms; the fresh JS control takes 17.4 ms.
Native selection has seven passes and one inconclusive timing result. All eight cases match frozen bytes.
The completed diffusion changes are now [PR #152](https://github.com/mia-cx/ditherette/pull/152), head `38cc3dd2`, on PR #148.
It adds alpha-only sink classification and budgeted exact byte-RGB caching to the earlier preparation and metric work.
The wider native selection passes all 24 cases. Three retained native trials total 160 workers and 10,880 exact samples.
Chromium Floyd-Steinberg at 50% now takes 258 ms versus fresh JS at 322 ms. Several adaptive cells still miss the target.
Combined validation passes 452 native, 46 interface and 13 private Wasm tests, both builds, the freeze guard and three-browser progress/source checks.

Sparse nearest `3b43fb43` now handles standalone resize and fused process under [issue #151](https://github.com/mia-cx/ditherette/issues/151).
All 96 Chromium six-sample qualification cases complete with exact baseline PNGs.
Warm public calls have 49 target passes, 46 misses and one borrowed identity no-op. Cold calls have 36 passes and 59 misses.
Nearest processing at 5% takes 1.75 ms warm versus JS at 7.4 ms; standalone nearest still loses.
Sparse nearest is now [PR #154](https://github.com/mia-cx/ditherette/pull/154), head `652b92a1`, on PR #152.
Measured code `5a08f2f0` uses compact row/column offsets and direct standalone output when progress is disabled.
All 18 three-browser PNGs match the compact-gather predecessor. Full processing at 5%, 10%, and 25% meets the cold/warm JS target in every browser.
Standalone resize still misses; Chromium 25% warm drops from 4.30 to 2.85 ms versus JS 2.80 ms.

Adaptive rows are [PR #155](https://github.com/mia-cx/ditherette/pull/155), head `7d775e8a`, on PR #154.
Native candidate `0ebcae59` versus `95aad503` passes all 35 exact gates across 140 serial workers and 11,200 samples.
Browser artifact `29cfdb8c` includes the equivalent pinned-nightly compatibility correction.
All 72 adaptive recipes complete. Warm target passes are 23/24 Chromium, 24/24 Firefox, and 17/24 WebKit.
All 24 Chromium PNGs and eight directly compared WebKit diffusion PNGs remain exact.
Integrated validation passes 464 native tests, 21 private Wasm checks, and scalar/threaded builds.
Native/browser raw evidence and provenance are archived in each delivery PR. No overall performance or release completion is claimed.
Larger nearest gathering is [PR #158](https://github.com/mia-cx/ditherette/pull/158), head `0400f1bc`, on PR #155.
Global candidate `520f2008` regresses Bayer palette edits from 66–76 ms to 148–166 ms and is rejected.
Selected `b72238d4` preserves separable/Yliluoma /16 eligibility while allowing standalone/direct/diffusion /4.
All 18 global-trial and nine selected-trial PNGs are exact. Selected 50% warm resize takes 9.05/9.5/7.5 ms in Chromium/Firefox/WebKit, versus previous 15.75/20/15 ms.
Bayer palette edits recover to 65.6/74.5/77 ms. Several JS cells remain unmet.

Row addressing is [PR #159](https://github.com/mia-cx/ditherette/pull/159), head `c92e4869`, on PR #158.
Isolated native `98240c4f` versus `0ebcae59` passes 35 exact gates, 140 serial workers and 11,200 samples.
WebKit 50% Floyd warm time falls 609.5 to 589.5 ms, meeting the declared 3% small-candidate target but still trailing JS 383 ms.
Joined runtime `753bfebe` passes 476 native tests with bench-subjects, 21 private checks and both builds.
A seven-line Yliluoma test-boundary snapshot fix restores inherited cache-hit assertions without runtime changes.
Final scalar SHA is `be2e868f9404a2bd1ad47420052b914526c296488d3539499dcae6029024078f`; threaded SHA is `51b9cee2512883ff8756119db1911050eb09484accba5f8a175271eb7749872c`.
All finished compiler targets from this work are cleared; native executables, Wasm snapshots, PNGs and raw evidence remain outside targets.
Mia approves an opt-level=3 production Wasm experiment, measuring speed and package size before selection. The default manifest remains opt-level=s.
The experiment completes 33 browser cases with exact before/after PNGs. The scalar-only change adds 21,236 bytes to the npm archive (5.1%). Area and Lanczos improve; Chromium Oklch's targeted repeat is 83.85→88.45 ms warm. See `docs/performance/wasm-profile-experiment.md` before selecting a default.
Mia keeps frozen anti-aliased bilinear unchanged. Historical JS samples four pixels; downscale comparisons retain timings but are labeled quality mismatches outside like-for-like target counts.
On 2026-09-12, Mia also keeps the legacy website adapter unchanged and deprecates JS area comparisons for two-axis upscales. JS substitutes bilinear; frozen and production Wasm use exact area coverage. Preserve timings as quality mismatches outside like-for-like target counts, not as Wasm wins. Downscale comparisons remain applicable. This resolves the area compatibility decision raised during PR75 babysitting, not its other review findings or merge gates.
The wider JS target remains open. No merge, release, or activation occurs.

Restarted agents own separate worktrees. `v1-diffusion-hotpaths` owns diffusion, quantizer matching/cache and `pipeline/indexed.rs`.
`v1-sparse-nearest` owns nearest plans, resize/processor integration and Wasm gather helpers.
The coordinator owns package JavaScript, evidence and this map. Measurements wait until implementation agents idle.
Sparse fused-process integration follows the standalone resize handoff and the release of `pipeline/indexed.rs` ownership.
No merge, publication or activation occurs.

### Scalar quantizer hot paths, 2026-09-11

[PR #146](https://github.com/mia-cx/ditherette/pull/146), issue #145, is open on PR #144.
Head `16fea204` on `perf/v1-quantize-hotpaths` retains the measured preserved-alpha row specialization.
Production changes total 23 added lines. Frozen code, resize kernels, shared image helpers and RGB cache logic stay unchanged.
Forced inlining and exact adjacent-RGB reuse were measured and removed after failing to improve browser controls.

The corrected Firefox copy enables optimizing Wasm. Older Firefox measurements used Juggler's baseline-only configuration.
Fresh full-size Celeste warm public calls improve from 108/120/119 ms to 83/86/83 ms in Chromium/Firefox/WebKit.
JS controls take 151/196/136 ms. These are public processing calls, not the earlier end-to-end numbers.
Eight trials retain 1,620 timed calls, 540 primes, 1,440 exact repeats and 54 exact PNG pairs.
Native evidence contains 132 serial workers and 2,552 samples, all exact against frozen outputs.
One native selection case stays inconclusive after its bounded repeat. Native promotion and release gates remain held.
The separate timed spec comparison passes all 15 full-image cases; the tiny-call setup regression remains visible.

444 native tests, 45 interface tests, both builds, the trusted freeze guard, three-engine scalar ownership/progress and two-engine threaded ownership pass.
The broader Yliluoma-oracle test stopped at setup because its separate oracle was absent; no pass is claimed.
For future performance work, read `docs/performance/quantize-hotpaths.md` on this branch and use corrected Firefox preparation.
Compact raw evidence is committed. Package snapshots and PNGs remain outside targets; finished targets and browser copies are removed after handoff.
No merge, publication or activation occurs.

### Dependency caching correction, 2026-09-10

[PR #144](https://github.com/mia-cx/ditherette/pull/144), issue #143, is open on PR #142.
Head `646de481` on `perf/v1-tiered-cache` includes measured source `1762aaa0` and the retained report.
Mia approves source revisions and stage dependencies instead of runtime image hashes. Frozen specifications stay unchanged.
Exact-byte source checks, identity pass-through and downstream-first lookups preserve output ownership, memory limits and successful-publication rules.
Small settings and palette records retain their existing compact hashes. Returned staged RGBA inputs verify retained bytes before adopting a dependency key.

Fresh before/after runs cover four Celeste scales and three engines: 432 timed calls and 144 primes, with 24 byte-identical rendered image pairs.
At 100%, warm end-to-end medians fall from 335/2107/321 ms to 174/918/154 ms in Chromium/Firefox/WebKit.
Firefox quantization and small-output cold overhead remain open. These bounded measurements do not clear release gates.
442 native tests, 61 benchmark-feature library tests, 45 interface tests, both builds, three-engine scalar checks and two-engine threaded ownership pass.
The trusted freeze guard passes. Finished compiler targets are removed after preserving package assets, raw samples and PNGs.
For the next performance change, read this PR's `docs/performance/tiered-caching.md`; historical hashing reports describe older revisions.
No merge, publication or activation occurs.

### Cold processing correction, 2026-09-10

[PR #142](https://github.com/mia-cx/ditherette/pull/142), issue #141, is open on PR #140.
Head `ea2c85bb` on `perf/v1-cold-processing` selects bounded exact RGB memoization, metric-specific matching, and scalar Wasm SHA-256.
The source measured is `60b47c26`; final changes only add the report and compact evidence.
Frozen code, landed resize helpers, the build profile, and public JavaScript API remain unchanged.

Native selection covers 31 cases, with one bounded CIELCH repeat clearing the initial inconclusive result.
The separate 16-case spec comparison retains its inherited tiny-call regression and noisy OKLCH circular-hue timing.
Across these runs, 192 serial workers produce 3,744 samples with exact outputs and clean shutdown.
Two three-engine browser experiments complete 891 calls without errors or expanded-output mismatches.
Celeste nearest plus sRGB changed-settings medians fall from 198/1357/258 ms to 33/207/33 ms.
First-image calls still trail historical JS. Firefox hashing does not improve; release holds remain intact.

Both package builds, 440 native tests, 44 interface tests, the trusted freeze guard, and scalar/threaded ownership checks pass.
All completed compiler targets are removed after preserving raw outputs, PNGs, runtime assets, and a 328 KiB audit archive.
See the PR's `docs/performance/cold-processing.md`. Nothing is merged or activated.

### Browser source reuse correction, 2026-09-10

[PR #140](https://github.com/mia-cx/ditherette/pull/140), issue #139, is open on PR #138.
Head `7a57caa5` on `perf/v1-wasm-overhead` retains an owned source snapshot only after success.
Mia approved exact-byte verification before identity reuse instead of mandatory per-call hashing.
The frozen reference, landed resize kernels, release profile, and deployment defaults remain unchanged.
Contiguous website crops now borrow source bytes before Rust takes its snapshot.

Two fresh scalar Celeste runs complete 810 calls across Chromium, Firefox, and WebKit with exact baseline outputs.
Repeated nearest plus sRGB processing at 650 × 1042 falls from 235/1132/229 ms to 11/15/8 ms respectively.
Cold and changed-settings calls still lose to historical JS. No existing release gate is relabeled or cleared.
Full native suites, both package builds, the trusted freeze guard, 44 interface, 35 adapter, and 16 worker tests pass.
Scalar ownership passes all three engines; threaded ownership passes Chromium and Firefox.
The implementation worktree's entire compiler target is removed after verifying 520 retained files and 23 candidate assets.
The report and raw evidence are in the PR's `docs/performance/wasm-source-reuse.md` and adjacent archive.
No PR is merged or activated. Next performance work must separate cold and cache-miss costs from repeat hits.

### Scalar spec/prod correction, 2026-09-09

The bounded scalar correction is delivered in two open, non-draft, unmerged PRs. Both have auto-merge disabled.

| Delivery | Head | Base | Progress |
| --- | --- | --- | --- |
| [PR #137](https://github.com/mia-cx/ditherette/pull/137), issue #135 | `impl/v1-scalar-spec-bench`, `b4b8dbea` | S45 `5da82d12` | 101-case tooling and full report ready |
| [PR #138](https://github.com/mia-cx/ditherette/pull/138), issue #136 | `perf/v1-scalar-packed-loop`, `cef9a733` | PR #137 `b4b8dbea` | Exact scalar field and packed-loop changes selected |

[The per-kernel report](https://github.com/mia-cx/ditherette/blob/impl/v1-scalar-spec-bench/.plans/83-scalar-measurements.md)
contains all baseline, selection, repeat, and final spec/production medians. Its audit counts 1,232 serial workers and 23,149 samples.
Fresh production selection measures 11.91x to 55.52x faster field recipes, 5.59x faster packed sRGB, and 6.52x faster YCbCr.
Every required selection case passes after one five-case repeat; recorded outputs are exact.
CIELAB forward is 8.54% slower within the agreed 10% limit. The noisy Yliluoma target candidate remains held.
The final spec-relative report still records five inherited resize differences, 25 slower-production cases, and five inconclusive cases.
None of those verdicts is relabeled. Native benchmark-profile results do not establish browser/Wasm speedups or release readiness.

Measured source `8e09c05d` remains on `evidence/v1-scalar-selected-8e09c05d`; held `086bbd47` has its own evidence branch.
Delivered processing inputs match the selected measured source exactly after rebasing. All 21 focused scalar release checks pass again.
The trusted freeze guard passes, including isolated native/Wasm compilation. Landed resize, color arithmetic, spec, and image files remain unchanged.
Verified archives preserve complete raw evidence and build provenance outside targets. Finished correction targets and nested freeze compiler outputs are cleared.
The disk has 101 GiB free. No owned benchmark, build, test, or implementation agent remains active.
Browser continuation stays paused; publishing, rollout, and visual-acceptance holds remain open.

#### Completed ownership and execution

Mia resumes the scalar optimization workflow after reviewing the missing direct spec/prod timing coverage.
Most earlier trials compare accepted and candidate production, with the frozen reference outside timers.
They remain evidence for those comparisons, not a complete scalar spec/prod report.
New work starts from final S45 `5da82d1221f79c2ddcb517eba9f621206468bfdc` and stacks above existing PRs.

| Owner | Branch and worktree suffix | Exclusive file scope |
| --- | --- | --- |
| Scalar benchmark agent | `impl/v1-scalar-spec-bench`, `v1-scalar-spec-bench` | Benchmark crates, subject adapters, focused tests, own plan |
| Converter reuse agent | `perf/v1-scalar-converter-reuse`, `v1-scalar-converter-reuse` | Production field/placement/Yliluoma loops, existing packed-space mapping, focused tests, own plan |
| Coverage agent | `docs/v1-scalar-coverage`, `v1-scalar-coverage` | Kernel/export coverage report only; no builds |
| Coordinator | `impl/v1-resize-integration` | Slice map, joins, fresh artifact preparation, exclusive measurements, retention and PR delivery |

Every measurement times actual frozen spec and actual production at matched boundaries, with named allocation/preparation scope.
Components and complete calls remain separate. All required scalar modes get coverage, with bounded representative workloads.
Fresh accepted/candidate comparisons decide optimizations; spec/prod comparisons show distance from the naive implementation.
Native scalar selection does not depend on unrelated browser noise. Browser validation and release holds remain separate.
Preserve landed optimized resize kernels, shared arithmetic, the frozen spec/image content, and existing exactness gates.
Known inherited resize differences remain diagnostic; no non-exact candidate is selected without Mia.
Stop agents and owned builds/tests before the single benchmark lease. Retain compact samples before clearing complete target trees.

The paused capped browser run drained Chromium successfully: four serial workers, twenty samples, exact outputs and a passing gate.
Its runner finished after 465.558 seconds with SIGTERM as the stop reason; Firefox and WebKit never launched.
The small report and complete compact Chromium record remain outside target; its SHA-256 is `1e36bd693c9d8d101e2615f658aec0b3426fb9f220f409d55d860df6d2ec1278`.
Whole capped and continuation target trees were removed after ownership ended, reclaiming about 1.55 GiB.
This pauses the remaining browser matrix, not the approved scalar correction. No PR is merged, published, or activated.

### S45 handoff, 2026-09-09

All 45 slices have open, unmerged PRs. S41 still has incomplete required measurements; the end-to-end goal remains unfinished.
S44 and S45 are prepared, not activated. Routine PR babysitting has not started.

S45 [PR134](https://github.com/mia-cx/ditherette/pull/134) is held at `5da82d1221f79c2ddcb517eba9f621206468bfdc`,
based on final S44 `76bf1f8938c14a1a4cde290d45c61a280f7434d2`. It is open, non-draft, unmerged, with auto-merge disabled.
All 45 preceding PR heads, including restoration, belong to this branch. Every published base belongs to its child head.
Website source `80c62ef5` passes 123 server and four Chromium checks; eight preparation checks pass.
Svelte checking has zero errors and one absent generated-types warning. No new retry UI is added.
The client replaces failed workers; the next processing request retries initialization in a fresh module registry.
Historical TypeScript benchmarks compile pinned Git revision `a895267baea624a6e89bfcef6c5147f170e8a8f7`.
All 24 emitted historical modules match S41. All 45 package distribution files match S43 tarball `78a3d5b7`.
Correction `d7b99198` removes a duplicate provenance field rejected by the actual Rust decoder.
The real decoder red/green proof and ten native protocol checks pass. The historical revision remains in the hashed compiler manifest.
Package/Rust production inputs, frozen spec/image/guard, and landed kernels are unchanged. No new performance measurement runs.
Compact reports remain in `.plans/87-retirement.md`, `87-retirement-evidence.json`, `87-benchmark-provider.md`, and `87-stack.json`.
The whole finished target tree is removed, including the 1.25 GiB native decoder build. Source and the small S43 artifact remain.
Next work is the bounded S41 missing scalar matrix continuation. Existing regressions and all human/operational holds remain open.

Continuation branch `impl/v1-s41-release-completion` starts from S45 `5da82d12`, without rewriting published parents.
Its plan checkpoint is `accc34e1`; the isolated matrix agent owns helper/artifact preparation in `.worktrees/v1-s41-release-completion`.
Root owns exclusive execution after that agent drains. The fixed budget covers Firefox 9 through 14 and WebKit 0 through 14.
Twenty-one complete cells require 84 serial workers, two alternating pairs each, within a 60-minute launch deadline excluding preparation.
There are no retries or sample-policy changes. Each completed cell preserves compact evidence before raw outputs are removed.
The runner checks at least 12 GiB free before each launch. No measurement starts during implementation or tests.
The final published-head audit includes all 46 PRs at `5da82d12`, with every current base in its child and auto-merge disabled.
Seven leftover nested spec-freeze compiler targets are removed, reclaiming about 1.4 GiB; no finished target trees remain.

The scalar continuation completes all 21 missing cells at source `2e85af6ba5b7efdedd7050e2fcf779ab2209964c`.
Its 84 serial workers produce 1,427 samples in 1,774.712 seconds. Eighteen cells pass, two are inconclusive, and one is incorrect.
Every completed cell is retained and reopened before deleting its raw directory. No incomplete attempts or unrun cells remain.
The 24 original scalar cells keep their original identities; the combined scalar matrix now has all 45 case reports.
The agent audits exactness and final gate details before the follow-up evidence PR. Coverage is not release acceptance.
The fresh tarball still matches S43 `78a3d5b7`. Firefox's task-local updater policy passes 60-second runtime immutability.
The source includes capped-only 256 KiB upload chunks. A small actual Chromium pipe test passes; full-size feasibility remains unrun.
Both implementation agents drain during the entire 29.6-minute measurement, then resume reports and capped-probe preparation.
Root retains only active native/public/capped artifacts for that next probe. Disk free space returns to about 101 GiB.

S44 [PR133](https://github.com/mia-cx/ditherette/pull/133) is held at `76bf1f8938c14a1a4cde290d45c61a280f7434d2`,
based on final S43 `15300c0dc461265fcef2bd72096de5202836706b`. It is open, non-draft, unmerged, with auto-merge disabled.
Its single runtime-line change prepares scalar Wasm as the website default, with a developer-only override.
The faithful temporary initialization fallback and cancellation stay intact. Seventy server and six Chromium checks pass.
No package/Rust changes, measurements, deployment, or activation occur. The finished worktree has no generated outputs.
S45 joins its isolated website and historical-provider branches before combined validation and PR134.
Browser TypeScript responsibilities remain. Both implementation owners have drained their jobs and returned their build targets.

S43 [PR132](https://github.com/mia-cx/ditherette/pull/132) is open at `15300c0dc461265fcef2bd72096de5202836706b`,
based on `impl/v1-s43-base` at `c43cea1269fcd666835d41c07d82a1c451604107`.
The audit includes every one of the 43 published prerequisite PR heads and 74 dependency edges.
A five-line fixture registration repair passes installed-package conformance without changing runtime code.
Fresh checks pass 49 native protocol, 40 JS/release, 42 interface, 2 staging, 4 glue, 68 website server,
6 website Chromium, 17 scalar conformance, and 23 automatic-policy checks. Full unchanged native/threaded checks reuse S42 evidence.
The freshly built tarball matches S42 `78a3d5b7` exactly. Build source stays `c43cea12`; tested fixtures are `f331ffcf`.
The S43 report retains all release blockers. Its finished target trees are cleared; a 1.6 MiB compact artifact bundle remains outside target.
S44 starts from this validated head in `.worktrees/v1-s44-rollout`, owned by its isolated implementation agent.
It prepares only a held default-rollout PR. No merge, deployment, or activation occurs.

S41 [PR131](https://github.com/mia-cx/ditherette/pull/131) is open at `d09e32df8e06ddddec3d6d374c6f22c280bc4d4b`,
based on S40 `95706738e4f3179824a66c80bb5728ce97a34b4b`. It changes benchmark infrastructure and reports only.
The batching candidate is rejected. Fresh historical anchors, admitted TypeScript comparisons, initialization,
and all twenty automatic cells complete. Automatic evidence has 80 workers and 1,318 samples.
Historical/TypeScript/automatic regressions, inherited drift, noise, 21 incomplete scalar release cells,
capped transport feasibility, and threaded WebKit remain release blockers. See `.plans/83-release-status.md` in PR131.
Forty focused merged JavaScript tests and fresh native/public preparation pass. The ordinary artifact stays byte-identical to S40.

S43 base `c43cea1269fcd666835d41c07d82a1c451604107` joins PR131 with S42 `db78ca0adf60d1a057d10c2bbe7aabc4c1781374`.
`impl/v1-s43-integration` owns integrated validation and the readiness report. `impl/v1-s43-stack-audit` owns the independent ancestry ledger.
Both use isolated worktrees. Root owns global progress and issue dependency tracking. No measurements run during these checks.
S41 returns its entire finished target trees, about 33 GiB, including raw payloads and browser snapshots.
Compact reports remain committed or archived under `benchmark-results/retained-reports-2026-09-09`.
Only the small reusable browser runtime trees transfer to active S43 ownership. Source and all PRs remain intact.

| Slice | PR | Branch | Immediate base | Head |
| --- | --- | --- | --- | --- |
| S35 | [126](https://github.com/mia-cx/ditherette/pull/126) | `delivery/v1-s35-resize` | `impl/v1-s34-threads` | `6bbe113b99a08f2a11ade6296ddf7f25e32e041b` |
| S36 | [127](https://github.com/mia-cx/ditherette/pull/127) | `delivery/v1-s36-fields-final` | `delivery/v1-s35-resize` | `b542bd94a5dbc724de73815ae0008a22985147fd` |
| S37 | [128](https://github.com/mia-cx/ditherette/pull/128) | `delivery/v1-s37-yliluoma` | `delivery/v1-s36-fields-final` | `91b114ba610588c504a7551e8123d72e36eb9e66` |

S35 starts from final S34 `d4531667e1158c2068f30614f40c9d39f8c5313e`.
The pure S36 publication excludes S37 implementation. Historical combined S36 remains preserved on its old branch.
Both downstream branches include their published parents. All production/build inputs at S37 equal tested `dc81818a`.
The ordinary tarball SHA-256 is `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
The trusted frozen guard and 23 automatic-policy browser checks pass, including nine cases per Chromium/Firefox engine.
Existing frozen bilinear drift and noisy performance comparisons remain S41 gates.

S40 base `5fccb9e6a51c6de49fd0051b204fb16bfde75e22` joins final S37 and S39 `5938b248506ae14d24471498c3a90bb42ed3c32a`.
Root owns `impl/v1-s40-conformance`, shared fixtures, CI, and delivery.
Memory fixture `e72cbe7b8a8eb56569a9ff76c1101a748999c180` joins from its isolated branch.
The S41 preparation agent only inspected existing matrices and evidence. No benchmark runs during conformance.

S40 is delivered in [PR129](https://github.com/mia-cx/ditherette/pull/129) at `95706738e4f3179824a66c80bb5728ce97a34b4b`,
targeting the explicit `impl/v1-s40-base` join. It adds crate-owned conformance commands and package CI.
The [report](https://github.com/mia-cx/ditherette/blob/96c281180529c3849823736581daebdcdbd8493e/docs/plans/ditherette-v1/s40-conformance.md)
records 424 scalar/425 threaded native tests, all three scalar engines, supported Chromium/Firefox pool lifecycle,
68 server and six website browser tests. Scalar memory stabilizes at 19 pages after warmup and 512 changing calls.
The local missing-library WebKit launch fails first; its isolated library-alias retry passes. Threaded WebKit remains blocked.
Fresh CI [34292675315](https://github.com/mia-cx/ditherette/actions/runs/34292675315) passes all conformance steps at this head.
The final CI-only fixes remove an invalid rustup flag, enforce Bash pipefail, and fetch locked dependencies before offline oracle preparation.
S35/S36/S37 return about 1.7 GiB of compiler outputs; S40 returns another 326 MiB. Artifacts and logs remain intact.

S41 and S42 start from that final S40 head. Their satisfied native GitHub blockers are removed; every issue remains open.
S41 matrix/TS adapters belong to `impl/v1-s41-performance`. Its separate cold-cache candidate belongs to `impl/v1-s41-cache-candidate`.
S42 owns `impl/v1-s42-package`, distribution and publication preparation only. No publication or benchmark is authorized during this phase.
S42 is delivered in [PR130](https://github.com/mia-cx/ditherette/pull/130) at `db78ca0adf60d1a057d10c2bbe7aabc4c1781374`, based on final S40.
Its [report](https://github.com/mia-cx/ditherette/blob/db78ca0adf60d1a057d10c2bbe7aabc4c1781374/.plans/84-package-report.md) records exact-artifact native, browser, package, and offline publication dry-run checks.
The immutable source is `bdbcb3c812701d50f157a12d7f157138f10b4013` and tarball SHA-256 is `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90`.
The tarball is 407,888 bytes. Only README and package metadata differ from the prior ordinary artifact.
Initial size-budget approval, metadata growth review, S41 gates, and human publisher setup remain pending. Nothing is published.
The root coordinator checks integration ancestry and tracking. All 41 implementation PR heads available before S42, including the restoration,
were ancestors of S40 at `96c28118`. S43 must repeat that check after S41/S42 delivery.

Current integration work continues on `impl/v1-resize-integration` in `.worktrees/v1-resize-integration`.
It owns the tracked progress table; the root table remains the visible mirror.
S21 public area/bilinear checkpoint `056a1324` joins at `b52d1c8b`, with native/package/three-engine conformance passing.
S22 public convolution checkpoint `f0976601` preserves native checkpoint `53eaf013` and passes all three package engines.
S23 is delivered in PR112 with exact shared-mip reuse. Its checkpoints and measurements appear below.
S24 is delivered in PR113. Its 124-worker measurement and remaining browser evidence appear below.
The S23/S24 join is validated at `1435642fc6b8f923fc4de4b4d1624b814940b5d1`; both delivered kernel families remain unchanged.
S25 adds all fifteen matching tags at `0085972a`, with 309 native tests and three-engine package conformance passing.
Its exact dispatch candidate `230046ff` is rejected after the fresh 276-worker comparison.
The run retains 5,133 timing samples and all verified outputs are exact; a required native score control regresses and several controls remain noisy.
The all-mode baseline remains selected. S25 is delivered in [PR114](https://github.com/mia-cx/ditherette/pull/114), targeting S24.
Current head `af8259ac766268e78690a569c10494c62cdb7ce2` carries both benchmark snapshot fixes without changing package code.
Its [measurement record](../../../.plans/66-measurement.md) retains both attempts, exactness scope, rejected candidate, and artifact identities. No further slice measurements are planned.
S26 literal field baseline `e156cfbf` reaches validated public checkpoint `089251287e387cb575e22e8993d8989a371a089d`.
It passes 13 scoped native, 24 interface, 11 private ABI tests, both builds, three browser engines, and the trusted freeze guard.
S26 benchmark protocol completes at `60516c6a12c62f90080b884df6918c1c287f86f3`, with 22 Rust and 28 Node checks passing.
Accepted S26 integration `3915f60519995cb9087a18b3bfd6bd7220ae804a` preserves the validated public implementation.
Converter candidate `b237b7468fa5fc349760bc0086bd1748113b6d82` passes native/public/frozen checks and independent review.
Its fresh comparison completes 208 workers and 4,048 samples. Untimed verifier repair confirms exact native controls.
Browser pair-noise gates remain inconclusive, so the original field implementation stays selected. No retry runs.
S26 is delivered in [PR115](https://github.com/mia-cx/ditherette/pull/115), targeting S25.
Current head `59036e1aef87943e462b4cce6b371e5edd082979` includes the updated S25 parent and unchanged package code.
The delivery rebase preserves measured runtime bytes. See [the measurement record](../../../.plans/67-measurement.md).
Combined resize/S26 join `82a7e3e9` passes 327 native, 26 interface, 12 private ABI, and all three installed-package engines.
Its [join record](../../../.plans/67-resize-join.md) records unchanged kernel bytes, adapter checks, and the validation tarball.
S27 public blue noise is delivered in [PR116](https://github.com/mia-cx/ditherette/pull/116), targeting S26 at `59036e1aef87943e462b4cce6b371e5edd082979`.
Its head is `c9666288cbe03a9f4dcfb14042cfcbff0fe61ca7` on `impl/v1-s27-bench`.
It passes native/private/interface checks, both builds, and all three package engines.
The [measurement report](https://github.com/mia-cx/ditherette/blob/c9666288cbe03a9f4dcfb14042cfcbff0fe61ca7/.plans/68-benchmark-results.md) records 68 reaped workers and 1,264 samples, all exact against the target-local frozen oracle.
All four runtime gates pass. Both roles rebuild measured source `44cbe43546e739f3d11f7b0bd08d7453afb83da1`; this establishes a baseline, not a speedup.
Native/Wasm rounding diagnostics remain intact. S41 still owns full-call bottlenecks and equivalent TypeScript comparisons.
S28 is delivered in [PR118](https://github.com/mia-cx/ditherette/pull/118) at `f4dfef7401d5474ac7318302d117ee0345449793`, targeting S27 `c9666288cbe03a9f4dcfb14042cfcbff0fe61ca7`.
The selected three-row diffusion path passes 360 frozen vectors in all three engines and the trusted freeze guard.
Its trial retains 128 reaped workers and 2,356 exact samples. All eight native comparisons pass with medians 8–91% lower than the literal full-image baseline.
Chromium and Firefox self-comparisons pass. Two WebKit self-controls remain inconclusive from noise or timer resolution, tracked for S41.
The [S28 report](https://github.com/mia-cx/ditherette/blob/f4dfef7401d5474ac7318302d117ee0345449793/.plans/69-measurement.md) distinguishes that release-evidence gap from the exact native optimization result.
S29 is delivered in [PR117](https://github.com/mia-cx/ditherette/pull/117) at `6eb9e00fd3191fc8bbd03559e89c67c762abfc25`, targeting the same S27 parent.
It retains measured literal `50cd96d17535ee7f81b1d7a63288751afab50e89`; converter candidate `fdb3921ae1cb7cc3834e42c204d61bb0a63c7cea` stays separate and unselected.
Both pass 367 frozen-Wasm fixtures per engine. The trial retains 128 reaped workers and 2,560 exact samples, but all four runtime gates remain inconclusive from paired noise.
The candidate has observed gains, not a confirmed regression. Its declared selection rule retains the baseline until required evidence passes.
Fresh public packages match their tested tarballs. S29 oracle executable sections also match; only candidate symbol metadata differs. Fresh-role conformance was not rerun.
All benchmark workers exited before implementation resumed. The [integration checkpoint](../../../.plans/68-70-checkpoint.md) retains validation and cross-target diagnostics.
S30's prerequisite join is validated at `22b6dd78a6e552596c34aa9e693ad74850426b23` on `impl/v1-s30-base`.
All six delivered prerequisite heads are ancestors. Both fresh builds, 38 native, 31 public, 14 private, 10 transport, and 28 protocol checks pass.
The fresh builds resolve the coordinator's stale S26 artifact failures. The coordinator joins this base at `cdef9f75`; its older staged artifacts remain historical.
[Issue #71](https://github.com/mia-cx/ditherette/issues/71#issuecomment-5582676111) records the exact dependency heads. Its six satisfied native blockers are removed; the issue stays open.
The runtime agent owns `.worktrees/v1-s30-process` and only the missing process composition, private ABI, and public wiring.
Readable native baseline `3335bb69acc6762a30a0b6844aef436c2e6b8de6` passes its initial all-family composition test, including metadata and one final output copy.
The frozen-only Process oracle is complete at `61d338431b5bd7039fa3d1fae4dd44200abdcfc5` in `.worktrees/v1-s30-oracle`.
Eight focused checks cover 130 resize/dither combinations, normalized identity, and output dimensions. Its isolated Wasm build passes.
The benchmark agent owns `.worktrees/v1-s30-bench`, complete-call adapters, and the fixed eight-case comparison against actual staged production calls.
Public runtime checkpoint `587339793cf70429b673e888a89d86a332541693` passes both builds, 42 scoped native, 33 public, and 16 private checks.
The coordinator's full trusted S18 guard passes. Independent native and private/public reviews find no actionable issues.
Fresh tarball SHA-256 `379c733b02bc67a24500d3ae825901d17d5fa342f93d114c20761da1aa9193b2` passes Chromium, Firefox, and WebKit.
Each engine verifies 423 actual Process/staged compositions plus inherited field, diffusion, and target-local Yliluoma suites.
Documentation head `963a80c56a6de8c36617cd08b14244c768b34e38` preserves those runtime bytes.
Final measurement source `e5aae7bf0e1761af2f970b6da75d34cf3a813323` joins benchmark head `834e882f95bb042b8356b324ef18343ef0ad1c52` without runtime changes.
Its full trusted S18 guard passes. Fresh native worker `9575273f58645c8d107f6df3848d49fbfde1ec0354ae7e54a51ecc28db6310b0` and the unchanged validated tarball are prepared.
The fresh Process oracle hash is `300f61644c4b7757d1ad80b97c515121a5ad241fa0051e9827e448f0067ffb64`.
Both roles use the same fresh artifact with distinct actual staged/Process calls.
Final conformance passes 431 identified fixtures and two area probes in each of Chromium, Firefox, and WebKit.
The fixed trial completes 128 serial workers and 2,504 samples. All 64 actual staged/Process output pairs match exactly.
All four aggregate gates remain `Incorrect` solely from inherited area reference differences. Five other timing cases remain inconclusive.
No confirmed greater-than-10% slowdown appears. No source optimization candidate is selected or implied.
The enlarged native area fixture has 125 inherited resize-byte differences, each at most one, which become seven indexed differences.
Frozen post-resize processing of the landed resize bytes equals both actual production call paths. Only that case may opt into diagnostic non-exact measurement.
Its frozen gate stays non-passing; Process-versus-staged equality remains mandatory. No new non-exact implementation or optimization is selected.
S30 is delivered in [PR119](https://github.com/mia-cx/ditherette/pull/119) at `88eb79fc129662fcfc6d4554d3109855348d0316`, targeting `impl/v1-s30-base` at `22b6dd78a6e552596c34aa9e693ad74850426b23`.
Only four plan/report files differ from measured source `e5aae7bf`; all six prerequisite heads remain ancestors.
The [S30 report](../../../.plans/71-benchmark-results.md) binds raw evidence, conformance, artifact sizes, and timing limitations.
S41 retains the timing/reference gaps. No complete pre-S30 tarball exists for a valid size comparison; S41/S42 retain that missing evidence.
S31 and S38 start from the actual S30 PR head in separate worktrees. Both keep `impl/v1-s30-process` as their immediate PR base.
The preparation owner has `.worktrees/v1-s31-preparation` and private cache/scratch/accounting changes plus focused tests.
The website owner has `.worktrees/v1-s38-website` and website adapter/worker mapping plus project-owned integration tests.
The benchmark owner has `.worktrees/v1-s31-bench` and cold/warm protocol, fixtures, and report only. It does not edit production or measure independently.
The coordinator owns this ledger, the slice table, issue availability, joins, and exclusive measurements. No benchmark is running.
PR119's [CI run](https://github.com/mia-cx/ditherette/actions/runs/34215724426) passes the exact-base guard but fails two controlled-mutation fixtures.
The coordinator reproduces both failures with `node --test --test-name-pattern='a new procedural macro dependency|resolved JSON feature changes' tools/spec-freeze/guard.test.mjs`.
The fixture copies the benchmark crate without its new `ditherette-bench-oracle` path dependency. Cargo fails before either intended mutation assertion.
Mia explicitly approves the fixture-only repair and continued implementation on 2026-09-08.
Approved trusted parent `af59df116193398886d1111964c87eaaa6111876` adds only the omitted crate to `guard.test.mjs`'s copy list.
PR119 head `d2356a502501b38ab4f3b476956fc90f1fbfec4a` joins that parent. All 11 mutation tests and the full trusted guard pass locally.
Frozen source, checkpoint, checker rules, and measured runtime are unchanged. New CI runs validate the approved parent policy.
Three GPT-6-astra high agents resume as `s31_runtime`, `s38_website`, and `s31_bench`. Their worktrees and ownership remain separate.
S31 checkpoint `0e90491500efcad950982a5b44df6013283c44aa` copies the frozen cache model literally, without public-runtime wiring.
Its 10 production baseline tests and 16 frozen cache tests pass. It retains one assigned 390 MiB worktree-local compiler cache.
S38 resumes from planning checkpoint `aeb48baa71ee1d64ba1d50eb6d29deda55b4e054` and implements typed package mapping and worker integration.
The S31 benchmark owner implements its eight-case cold/warm matrix and untimed setup/teardown hooks from `88eb79fc`.
Both approved-fixture CI runs pass. S30 review fixes advance PR119 to `f408bc99a80d3c83b6caee0b5c1d19868f0db876`.
The memory fix `aa4f78d1` stops charging an unused converter for non-separable Process recipes; five native Process tests and CI pass.
The provenance fix `86a98935` prevents sequential browsers from replacing original native probe references.
All 431 references and two area probes pass on each engine. The [review follow-up](../../../.plans/71-review-followup.md) records refreshed hashes without replacing measured evidence.
The runtime checkpoint `4aad1dbe` wires preparation reuse across all five methods; seven private and 24 focused native tests pass.
Independent review found transient diffusion scratch capacity omitted during growth. The runtime owner fixes this before measurements.
The benchmark checkpoint `9146aac5` implements cold/warm lifecycle and verifies every native/browser sample outside timing.
It joins the final S30 parent before fresh accepted-role builds. No benchmark is running.
S38 is delivered in [PR120](https://github.com/mia-cx/ditherette/pull/120) at `34ecca9063f68ccbbcb93a2e6d363bf57baa1129`, based on final S30 `f408bc99`.
Its 47 focused tests, four Chromium fixtures covering 72 mode combinations, and production build pass.
Website mapping uses the actual shared TS RGB strength constant, 96, divided by 63.75. Historical frozen notes remain unchanged.
Website generated-output cleanup reclaims 8.8 MB. No Rust compiler output belongs to that completed worktree.
S32 planning checkpoint `4a2b5673216921648188f2de5c79753b7d5e0d91` lives in `.worktrees/v1-s32-stages`; no runtime work starts before S31 delivery.
The [stage-cache plan](../../../.plans/73-stages.md) reuses the literal cache model and S31's shared store. It creates no unnecessary full-image color buffer.
Restored S30 blockers are removed after descendant ancestry verification. Remaining S31 dependencies stay blocked until delivery.
Accepted benchmark head `863889e52f1b752b6adfc22a9c775b3823f2997e` retains S30 runtime and adds the shared preparation benchmark protocol.
Fresh native/public accepted artifacts live in the benchmark worktree under `target/s31-baseline-863889e-native` and `target/s31-baseline-863889e-public`.
The S31 candidate joins that protocol before its own fresh builds. The trusted frozen guard passes after moving reference comparisons into integration tests.
S31 validated runtime is `972d4e9a5882b25bca3de5f0786ad1525b5e6329`; fresh native/public artifacts bind that clean source.
The corrected installed-package suite passes on all three engines. Its bounded fixture now includes cache-control records and still rejects the 40,400-byte source.
The exclusive trial finishes all four runtimes with exact outputs and no confirmed greater-than-10% slowdown.
Chromium/Firefox gates pass; native warm Process and WebKit cold/warm Lab plus cold Lanczos3 remain inconclusive.
Warm Lanczos3 improves across all runtimes. Retain the required preparation baseline without another candidate or retry.
Evidence lives at `.worktrees/v1-s31-preparation/target/s31-trial-01`; only `*-prepared-v2` snapshots were measured.
The benchmark owner writes the report while the runtime owner prepares its stacked PR. No benchmark remains active.
This validated-runtime handoff permits S32 to begin on `972d4e9a` while S31's report-only PR handoff finishes.
The S32 owner rebases its plan checkpoint and owns image-stage identity/store/pipeline integration in its isolated worktree.
S31 is delivered in [PR121](https://github.com/mia-cx/ditherette/pull/121) at `a3c9629f35280c36e838faa00e9b664b23abcb53`, based on final S30 `f408bc99`.
Frozen-reference CI passes. A resolved rebase conflict preserves exact tree equality and the measured `972d4e9a` ancestor.
The [S31 report](../../../.plans/72-benchmark-results.md) verifies 128 matching starts/reaps, maximum live worker count one, 64 exact pairs, and 2,560 samples.
Its [four inconclusive cases](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5587432141) remain S41 work.
Raw threaded Wasm grows 10.47%; the [size-review item](https://github.com/mia-cx/ditherette/issues/84#issuecomment-5587431766) remains explicit for S42/S43.
S32 checkpoint `4e9baaad80d6e590303dcdfed5346f01f259b8ce` includes final S31 ancestry and passes exact stage-identity and owned-metadata tests.
Issue #73's S31 blocker is removed after verifying that ancestry. Its same-store transaction and pipeline wiring remain in progress.
The runtime owner uses `.worktrees/v1-s32-stages`; the public-fixture owner uses `.worktrees/v1-s32-public`; the benchmark owner uses `.worktrees/v1-s32-bench`.
Benchmark checkpoint `64eb3357` declares four cold/warm workloads and verifies per-sample priming helpers. No S32 measurements have run.
Store checkpoint `b6193dd3f14aaabdd4077e7170ad8b5282c8e98d` passes 22 native library tests and an independent read-only review.
It shares preparation/image caps, LRU, pinned transactions, and success-only publication. Processing-path wiring remains in progress.
Public fixture checkpoint `8bd9208138322776433cb0eb3e9dcf0f4d15de52` passes the actual installed S31 package in all three engines.
It covers five methods, 18 compositions, mutation, metadata ownership, disposal, and failed final-copy recovery.
Those checks establish observable behavior on the recorded S31 tarball, not private cache hits or S32 artifact conformance.
The public-fixture owner returns its completed checkpoint and independently reviews runtime checkpoints without editing the runtime worktree.
PR121's later review finds retained byte scratch can overlap its replacement allocation. It also flags parsed Process diffusion-policy accounting.
The public-fixture owner switches to targeted fixes and physical-allocation tests in `.worktrees/v1-s31-preparation`.
Only its new `target/compiler-review` belongs to this review task. Historical benchmark artifacts remain unchanged.
S31 availability is temporarily withdrawn and #73's blocker restored until the corrected parent joins and validates in S32.
S32 runtime/protocol work continues in isolation; neither prepares measurement artifacts against the outdated parent.
S31 fix `59b1fe3acdbeae27bbb8ab780b46d2b6b9d67a76` resolves both findings with independently failing tests before each fix.
All 28 focused native tests and frozen-reference CI pass. Four review threads are replied to and resolved.
The [follow-up](../../../.plans/72-review-followup.md) records physical allocation evidence and the conditional parsed-policy charge.
Historical S31 measurements remain bound to `972d4e9a`; the fixed-size workloads do not exercise retained-buffer growth.
S31 availability is restored. S32 removes its blocker after joining and validating this corrected parent.
S38 fix `0305456bc25259a92d46ded245ae09aaf407be07` caps persisted adaptive radii at the package maximum.
All 51 mapper/worker tests, focused lint, and CI pass. Its review thread is resolved; no package, kernel, or UI changes occur.
These three owners have disjoint source/test/protocol responsibilities. S31 compiler caches are no longer assigned to any agent.
S32 runtime `d638f87c3a16824ef52964bbb611ef91b173f2ee` joins corrected S31 and benchmark protocol `d51a70a2daf054357d16ea66b235da3733c02888`.
Full native tests and the trusted frozen guard pass. Image-stage entries reuse the existing shared store and landed kernels.
Fresh accepted artifacts bind `d51a70a2`; candidate artifacts bind `d638f87c`. Test-only follow-ups do not change those identities.
Both installed packages pass Chromium, Firefox, and WebKit, including focused ownership and broad conformance suites.
The four completed trials remain under `v1-s32-stages/target/s32-trial-01`. Every recorded comparison is exact.
Cold resize regresses 11.8-68%; browser cold Process regresses 21.5-26%. These confirmed release blockers remain S41 work.
Warm paths improve substantially, but inconclusive cases remain explicit. No extra tuning or retry is selected.
S32 is delivered in unmerged [PR122](https://github.com/mia-cx/ditherette/pull/122) at `127a0428a0bfdad7ea3e239e6a96f375449815bb`, based on corrected S31 `59b1fe3a`.
The coordinator join preserves exact code equality; its only conflict replaces the old S32 planning text with the completed delivery plan.
The [report](../../../.plans/73-benchmark-results.md) verifies 128 reaped workers, 2,560 samples, and 64 exact role pairs.
Seven regressions and five inconclusive cases remain [explicit S41 work](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5588316158).
S32 availability is recorded and S33's blocking edge removed. No issue or PR is closed or merged.
S33 runtime and installed fixtures branch from that same validated handoff in `v1-s33-progress` and `v1-s33-public`.
Runtime checkpoint `5a832320` reuses the copied lifecycle model and passes two focused controller tests.
Kernel checkpoint `0acb7842` passes the full native suite and five progress fixtures. Existing arithmetic, traversal, and scratch ownership remain unchanged.
Installed fixture checkpoint `0bb2552c` passes shared-runner checks on S32 and intentionally fails at its unsupported callback guard. Actual S33 validation remains pending.
The separate `v1-s33-bench` plan fixes five public workloads and separates disabled-support cost from callback-delivery cost.
S33 delivery checkpoint `06d9ad0730669dac3008baf848b5eb463689c584` validates the complete callback contract.
Measured candidate `4a75d479d38a92c75e8ff4ed96c916fec3aaf8f4` passes focused 8/8 and broad 4/4 installed suites in all three browsers.
The trusted frozen guard passes after moving a reference-comparison test outside production. Frozen content and policy remain unchanged.
Six serial trials finish under `v1-s33-bench/target/s33-trial-01`; every recorded comparison is exact.
Callback overhead passes all three browsers. Disabled-support comparisons pass Firefox and WebKit; Chromium Lab76 and Lanczos3 remain inconclusive.
The report owner now owns `v1-s33-progress` for evidence and the stacked PR. No runtime edits or repeat measurements are planned.
S34 runtime and public fixtures start from validated `06d9ad07` in separate `v1-s34-threads` and `v1-s34-public` worktrees.
The runtime owner has package/glue/pool implementation and native/private tests. The fixture owner has installed browser tests and test-server support.
They coordinate shared fixture routes before editing. Root owns tracking, joins, and startup benchmark preparation.
Only the new S34 runtime compiler targets are assigned. S33 targets return for cleanup after its PR handoff.
S33 is delivered in unmerged [PR123](https://github.com/mia-cx/ditherette/pull/123) at `b2ca677ed9927165a1010f5c52646a989d8a02ca`, based on S32 `127a0428`.
The coordinator joins it without conflicts and verifies identical crate, package, and script code against that head.
Its [report](../../../.plans/74-benchmark-results.md) verifies 120 reaped workers, 2,400 samples, and 60 exact role pairs.
All 15 callback cases pass. Chromium's disabled Lab76 and Lanczos3 comparisons remain inconclusive for S41.
Those gaps are recorded in [S41](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5588948095).
S33 has the availability label. S34's [validated prerequisite handoff](https://github.com/mia-cx/ditherette/issues/76#issuecomment-5588948351) removes its satisfied native blocker while preserving the original dependency record.
The returned report owner now owns S34 benchmark protocol and startup artifacts in `v1-s34-bench`.
Its amended fixed plan has 40 serial workers across scalar regression and threaded same-artifact controls. No measurement is running.
S34 fixtures at `1b4bc28cd185b4987e1b251c1ac72dcbdbc16837` pass selection and partial-start cleanup in all three engines.
Chromium and Firefox pass actual disposal and host termination. Pinned WebKit 26.4 retains atomic-wait workers.
The independent upstream Wasm reproduction fails without Ditherette or Rayon. WebKit fix `319508@main` needs verification in a recorded engine.
Keep this unresolved release gate and the failing lifecycle assertions. Unreliable capability probes are discarded, not shipped.
WebKit threaded startup is blocked and unmeasured because retained pools invalidate trial isolation. Scalar startup still covers all engines.
The [release gate](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5589175119) retains the exact fixture checkpoint and upstream reference.
S34 `29bccaa5` passes the trusted guard. Build source `2afd1802` adds only the startup-plan amendment.
Detached `v1-s34-accepted-source` at `bf7912db` and `v1-s34-measured-source` at `2afd1802` preserve exact build provenance.
S39 starts in `v1-s39-website` from the explicit S34 `29bccaa5` and S38 `0305456b` join `6230326d`.
Its owner changes website scheduling/fallback and focused tests only. Final S34 ancestry and artifact validation remain required before delivery.
S39 checkpoint `45947d96` adds immediate stale rejection, debounced replacement, and real progress forwarding; 54 focused tests pass.
S34 [startup attempt 01](../../../.plans/76-attempt01.md) retains 480 exact scalar samples from 24 completed workers.
The first threaded worker fails untimed preflight; all 25 started workers are reaped. No threaded samples exist.
An isolated reproduction proves main-thread `Atomics.wait` is forbidden and failed cleanup masks that error.
Runtime capability checks and real threaded fixtures now target blocking-capable host workers. The benchmark owner adapts the same collector's host context.
S39 resumes faithful initialization-only fallback while both S34 fixes proceed. Measurements remain stopped.
S39 `fa43ab4b` commits page-session faithful fallback. Its focused tests and six actual-package Chromium checks pass.
Final S34 ancestry and exact final-package validation remain before S39 delivery.
The returned S39 owner starts S35 native row-band adapters in `v1-s35-resize` from capability checkpoint `6296c66b`.
This owner changes only resize/color adapters, focused native tests, S35 subjects, and its plan. Its sole new compiler assignment is local `target/compiler`.
S34 retains loading/lifecycle ownership. Final S34 loader fixes and host-worker benchmark protocol must join before S35 artifacts or delivery.
S34 now ships as unmerged [PR124](https://github.com/mia-cx/ditherette/pull/124) at `d4531667e1158c2068f30614f40c9d39f8c5313e`, based on final S33 `b2ca677e`.
Trial 02 completes 40 started/reaped workers, 800 samples, 2,453 warmup calls, and 20 exact actual role pairs.
Four case gates pass; six remain inconclusive. No confirmed >10% startup slowdown is observed.
The pinned WebKit worker-cleanup failure and missing required-thread startup cell remain S41 release gates.
S39 is unmerged [PR125](https://github.com/mia-cx/ditherette/pull/125) at `5938b248506ae14d24471498c3a90bb42ed3c32a`.
Its explicit base `impl/v1-s39-base` at `2a0237680fa249e2293b991ca067d7baf05aef14` joins final S34 and S38 `0305456b`.
Final retained S34 tarball `33a46ac0de03c1d9947302af356648549cd288f8cfcbf5b3953843af65d75c9d` passes 68 focused server tests and six Chromium browser checks.
The rebase preserves S39's head and tree. The website flag remains disabled; no compiler outputs belong to S39.
S35 resumes from clean `20fc297b` with sole ownership of shared execution-policy and private Wasm wiring, resize adapters, and the pooled executor.
S36 resumes from clean `7e5689ca` with quantize/field adapters and direct/separable indexed integration.
S37 starts at final S34 `d4531667` in `v1-s37-yliluoma`, owning Yliluoma adapters and its benchmark fragment.
All three agents use GPT-6-astra at high reasoning in separate worktrees and own only their local `target/compiler`.
S36 coordinates Yliluoma indexed callsites with S37. Root owns final joins, tracking, package artifacts, and exclusive measurements.
The browser/native performance discussion is an aside. Keep same-kernel timing and compilation-warmup diagnosis in S41; continue slice implementation.
Combined row-band candidate `b6522e2f` in `impl/v1-s35-37-bench` includes S35 `87d69cc6`, S36 `fef1eafe`, and S37 `be989cc5`.
The join preserves S35's scratch-before-LRU pressure ordering and multi-stage private policy updates, plus both indexed adapters.
Its 39 library and ten focused threaded integration tests pass, including combined resize/indexed Process and mixing failure recovery.
The trusted frozen guard passes with unchanged checkpoint/digest. Developer-only policy metadata binds actual public host calls; 27 protocol and 24 timing/policy tests pass.
Explicit benchmark-feature build preparation passes six Node and ten native provenance tests. Normal package build defaults and exports remain unchanged.
The host fixture `dcfd5d72` and fixed 400-worker matrix `37b8bf59` join at clean source `2b6edc9c91307799e3f5ae16194ee0dfd3e5db38`.
Fresh native and benchmark-feature public builds pass. Tarball SHA-256 is `f58b0949f93486c0e69b4e956e70f4a6e3381299a08cc47f9ca18fde23d069a8`.
Seven actual host-worker fixtures pass in Chromium and Firefox, covering both row policies, progress, callback recovery, and durable outputs.
The fixed first sweep stops at Chromium warm bilinear's cache-prime check after 28 successful workers; all 29 started children exit.
Its strict prime check rejects inherited frozen-reference drift despite explicit diagnostic mode. No new production difference is established.
Raw evidence remains in `v1-s35-37-bench/target/rows-trial-01`; Firefox has not started. No configuration is selected from this incomplete run.
The resize owner fixes only same-call prime diagnostics and bounded error text. The fields owner summarizes completed pairs read-only.
Root owns fresh artifact preparation and exclusive measurements after that fix. All three slice implementations remain in progress.
Corrected source `5d16c5682f354fecd75ca7f761802d9e2ea75ab5` preserves every production file and the same tarball digest.
Same-call prime diagnostics retain unstable outputs; strict and different-stage checks remain unchanged. New machine JSON omits indentation.
Fresh native/public artifacts and the seven host fixtures pass again. `target/rows-trial-02` prepares the unchanged 400-worker matrix.
Three clean delivery branches preserve separate slice ownership and immediate-parent ancestry:
`delivery/v1-s35-resize` at `2bd25aed`, `delivery/v1-s36-fields` at `4c4ea884`, and `delivery/v1-s37-yliluoma` at `d8834596`.
The complete S37 delivery tree equals corrected measurement source `5d16c568`. Policy selection and PR reports await complete evidence.
Trial 02 completes all 200 Chromium workers. Its 50 case gates are 37 pass, nine inconclusive, three inherited bilinear mismatches, and one small-nearest regression.
The complete-call large four-worker results show about 26% lower Lanczos3 latency, 49% lower sRGB field latency, and 67% lower adaptive Yliluoma latency.
These compare forced policies within the same threaded artifact, not threaded Wasm against the ordinary scalar build.
Firefox stops after 15 reaped workers when the browser creates writable `.parentlock` and `updates/` inside its immutable runtime snapshot.
The files are absent from the source snapshot; the normal Playwright profile is separate. Root preserves the failed snapshot unchanged.
The Firefox updater caused that runtime write. A fresh runtime copy binds its own update-disabled policy through Playwright's alternate policy path.
Compiled browser files remain unchanged; the configuration files enter the normal immutable snapshot digest. Historical snapshots remain untouched.
Replacement trial 03 finishes all 200 Firefox workers with maximum live count one and no owned processes remaining.
All 100 actual scalar/row pairs in each engine match byte-for-byte, including metadata. The separate reference probes retain inherited differences.
The two completed engines collect 7,215 of 8,000 requested samples. Their declared time cap shortens 64 workers across 17 engine-cases after the existing minimum sample count.
Its 50 case gates are 43 pass, four inconclusive, and three inherited bilinear mismatches. The coordinator exits 2 for those gates, not a worker failure.
Firefox's large four-worker latency falls about 37% for Lanczos3, 50% for sRGB fields, and 67% for medium adaptive Yliluoma.
The Chromium report digest is `160255130439e20dbbcd46a224f4911bed8fc5c10cc3840f46627a09e2148642`.
The Firefox report digest is `77af173eb9b0c2203d8ad43a75181a68e90d57cf2a8388c1e7b1784603594e73`.
Both reports compare forced scalar and row policies in the same threaded tarball; ordinary scalar-build comparisons remain S41 work.
All three agents resume in the clean delivery worktrees. S35 owns the shared automatic-policy seam and resize selection.
S36 owns combined evidence and field selection; S37 owns mixing selection. Only common-engine measured configurations can become automatic.
Small or unmeasured classes remain scalar. Noisy cache-hit controls stay visible as incomplete S41 evidence.
The completed benchmark and compact-JSON build caches return to the coordinator. Cleaning their two audited compiler directories reclaims about 4.3 GiB of disk blocks.
Native executables, the tarball, both report digests, and all historical trial snapshots remain unchanged. Review can rebuild the compiler outputs.
Future predeclared sweep budgets must include untimed per-sample priming. Large Firefox scalar cache-hit controls took about nine minutes each.
Wait for benchmark exit without repeated progress-counter polling. Resume implementation only after the benchmark and its children exit.
Candidate provenance uses the clean detached `v1-s32-measured-source` checkout at its exact built revision.
Issue #73's corrected S31 blocker is removed after ancestry and native validation.
Mia defers routine review and babysitting until the full implementation stack exists. Inline fixes address implementation blockers, correctness failures, and architecture that would propagate downstream.
The coordinator owns benchmark protocol/adapters, joins, and exclusive measurements.
A separate owner adds native budgeted subjects in `impl/v1-resize-bench-subjects`; no production files belong to that task.
S21/S22 measurements complete all 304 serial workers and retain 5,760 samples. No measurement is running.
All native production pairs preserve landed bytes, with no slowdown above 10%; no kernel retuning is selected.
The [S22 measurement record](../../../.plans/63-measurement.md) links artifacts and records each convolution median.
Public TypeScript differences and complete-call costs remain [S41 work](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5575617483).

## Compiler-output cleanup

The first inactive-worktree cleanup reclaimed about 46 GiB from 124 compiler profile directories.
After S27 preparation and measurement ended, eight returned S24 native/S23 Wasm profiles reclaimed another 6.55 GiB.
The disk had 57 GiB free afterward. Source, copied binaries, trial snapshots, reports, and custom target evidence remain intact.
After S29 delivery, eight returned S22 compiler profiles reclaimed another 5.34 GiB. Disk free space was 60 GiB afterward.
Three returned freeze-checker profiles reclaim another 0.29 GiB. Their custom evidence and the active trusted checker cache remain intact.
The returned S30 oracle compiler cache reclaimed another 1.29 GiB. Its separately retained Wasm reference artifact is unchanged.
Cargo clean refused the missing root `CACHEDIR.TAG`; cleanup instead removed three explicit, fingerprint-verified compiler profiles.
After PR119 opened, ten returned S24 quantize/S30 benchmark compiler profiles reclaimed another 10.91 GiB. Disk free space is 66 GiB.
After the S30 review fixes, its returned `target/compiler-review` cache reclaimed another 418 MiB of disk blocks.
Current S31 builds bring worktrees to 47 GiB with 60 GiB free; active caches remain assigned until PR handoff.
S31 handoff returns six compiler targets; their cleanup reclaims 6.93 GiB of disk blocks without changing retained artifact/report hashes.
The returned trusted-checker target reclaims another 369 MiB. Future checks rebuild it when needed.
The completed S31 review target reclaims another 516 MiB. Its retained package digest stays unchanged.
The S38 review recreates and then removes 100 KiB of generated SvelteKit files; no Rust outputs are created.
The completed S32 trusted check returns its compiler target. Cleanup reclaims 287 MiB of disk blocks from 695 rebuildable files.
Its exact realpath and compiler-only contents were checked; no active process owned that target. Frozen source and benchmark evidence remain intact.
After PR122 handoff, six returned S32 compiler targets reclaim another 6.18 GiB of disk blocks.
After PR123 handoff, six returned S33 compiler targets reclaim 5.51 GiB of disk blocks.
After PR124 handoff, root cleans six returned S34 compiler targets and the idle trusted-checker target.
Cargo reports approximately 6.4 GiB of logical file sizes removed. Both final retained package and worker hashes remain unchanged.
The joined row-band guard later recreates the checker target. Active row-band compiler directories remain assigned and untouched.
After their native tasks finish, four returned S35/S36/build/matrix compiler targets reclaim about 7 GiB of disk blocks.
Their paths contain only compiler profiles, have no symlinks or active owners, and are cleaned with explicit Cargo target paths.
All candidate source, installed artifacts, and partial-trial evidence remain intact. Future review rebuilds those returned targets.
The returned trusted-checker target reclaims another 289 MiB. Both reports and both package hashes remain unchanged.
Copied binaries, oracles, trial snapshots, and raw results remain retained. Compiler outputs can be rebuilt for review.
Current S34 runtime and benchmark owners use only their new worktree-local targets; public fixtures own no compiler cache.
Both retained package hashes and both report hashes stay unchanged. Copied binaries, oracles, trial snapshots, and raw results remain available.
S33 owns only its new worktree-local caches. The old S32 compiler assignments have ended.
Both owners drained their jobs. Exact realpaths, fingerprint directories, and process ownership were checked before deletion.
Eight retained tarball, binary, conformance, and report hashes remain unchanged. No complete target directory was purged.
The entries above record each cleanup separately. New slices own only their explicitly assigned worktree-local compiler outputs.
The coordinator also removes the 98 MiB syntax-checker profile created by the CI reproduction after its jobs drain.
All other completed-slice compiler ownership has ended. [S41 retains the measured release gaps](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5582606062).
Each completed PR returns its compiler outputs for cleanup. Review rebuilds them when needed.

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
S21 is open in [PR #110](https://github.com/mia-cx/ditherette/pull/110) at `2d5412664ccd27b5790db7493f375e3490e42c90`.
S22 is open in [PR #111](https://github.com/mia-cx/ditherette/pull/111) at `9eecc670d9ff587ff10f8d2f3a8b86bab600c988`.
These heads correct private ABI documentation and public benchmark stability checks without changing processing bytes.
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

## S23 exact trilinear

[PR #112](https://github.com/mia-cx/ditherette/pull/112) is open, non-draft, and unmerged, with auto-merge disabled.
Branch `impl/v1-s23-trilinear` targets S22 at `9eecc670d9ff587ff10f8d2f3a8b86bab600c988`.
Delivered head `cd7a0d298755818f86d710bef9f094c815d127de` includes that parent and unchanged measured processing bytes.
The coordinator includes this complete restack; benchmark snapshots remain bound to their original source revisions.

Literal missing-implementation baseline `fba85a94` precedes prepared baseline `0be73eb6` and shared-mip candidate `07d528a6`.
The candidate computes common storage-rounded mip levels once, preserving each arithmetic and RGBA8 reconstruction step.
[Fresh measurement evidence](../../../.plans/64-measurement.md) retains 80 reaped workers and 1,344 samples across native and three browsers.
All production comparisons are exact; all four runtime gates pass. Fractional call latency falls 27–56%.
Control cases remain within the 10% gate. The shared-mip candidate is selected; landed resize kernels remain untouched.

Final checks pass 18 focused native, 18 interface, eight private ABI, nine JS protocol fixtures, both builds, and three engines.
The trusted freeze guard and formatting pass. Earlier candidate validation also passes the complete 300-test native suite.

## Current benchmark verification correction

PR110 review found that browser timing retained only endpoint outputs, missing transient A/B/A changes.
Correction `debaa849f9041657ccc9b6942f561213d25d5dbc` is now included in PR110, PR111, and PR112.
It passes 22 JavaScript and 15 Rust checks; all three restacked trees pass their 22 JavaScript fixtures.
The collector verifies every durable result outside operation timers, with a bounded retained throughput batch.
It fails closed if the calibrated batch cannot fit the retention bound; it never silently lowers the iteration count.
Retention changes GC pressure, so new measurements need fresh artifact snapshots for both roles.
Historical evidence remains tied to its original collector and does not certify every intermediate output.
S24 adds exact indexed-result observation and completes fresh replacement benchmark snapshots.

## S24 packed-color quantization

[PR #113](https://github.com/mia-cx/ditherette/pull/113) is open, non-draft, and unmerged, with auto-merge disabled.
Branch `impl/v1-s24-bench` targets S22 at `9eecc670d9ff587ff10f8d2f3a8b86bab600c988`.
Current head `f8a2cc11dc42e4815ea8cffbb1b3f36c1a116395` preserves measured source `f4b90ecfcde63531fb992cf87ebda04d4e373032` processing bytes.
It snapshots first/distinct evidence outside timers and rejects shared result backing; 30 relevant Node checks pass.
All three addressed review threads are resolved. The current head is approved and all reported checks pass.
Fresh trials must include this collector. Historical measurements below are not reruns of the corrected protocol.
Literal baseline `a23260ed` remains in ancestry. The prepared integration reuses landed forward conversion equations and tables.

[Measurement evidence](../../../.plans/65-measurement.md) records 124 reaped workers, 2,463 samples, and exact recorded comparisons.
All 13 native cases pass. Full-call latency stays within 3% of the fresh literal baseline; no new kernel speedup is claimed.
Public accepted and candidate roles use identical artifacts. Firefox and WebKit controls pass.
Chromium's linear-RGB control remains inconclusive from pair noise; no retry extends the fixed worker budget.
[S41](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5576162129) retains that control and Firefox's undiagnosed absolute call cost.
Processing validation passes 307 native tests, nine private ABI tests, 20 interface tests, both builds, and three installed-package engines.
Benchmark validation passes 56 Rust and 26 controlled JavaScript tests. The trusted freeze guard passes after restacking.

[Combined-tree validation](../../../.plans/65-s23-join.md) passes 315 native, 59 benchmark, 26 controlled JS, 22 interface, and 10 private ABI tests.
Both builds, three-engine installed-package checks, and untimed benchmark adapter conformance pass.
The join only corrects a stale trilinear-unavailable test; it does not change processing or timing implementations.

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
