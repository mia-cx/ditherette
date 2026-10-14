# Current stable-release qualification, 2026-10-14

Ordinary npm publication remains blocked. Current conformance passes, but size review,
performance qualification, capped feasibility, and WebKit threaded cleanup remain open.
This report reconciles [#359](https://github.com/mia-cx/ditherette/issues/359).
It changes no runtime, workflow, size budget, frozen reference, or release hold.

This is a read-only inventory. No builds, processing benchmarks, or new timings run here.
Mia gives PNG export the first exclusive measurement window after correctness/build readiness.
The parent schedules every experiment below after PNG and owns integration and deployment.

## Current identities and reusable evidence

The report starts from clean `origin/main` at `c16ba62a5be0929c22aec84be868aef3c3fa1508`.
That includes direction PR #377. This report's eventual Git head identifies documentation,
not the package build source.

| Item | Exact identity |
| --- | --- |
| Release source | `e1df9fdc3687ff2812632aecc6807a84db3fbf08`, merged release PR #368 |
| Workflow runner head | `2dac2a2e2f398f728632cf5c73e3310f4dd9a363` |
| Package | Ordinary `ditherette@0.1.0`, public build, 54 installed files |
| Publication and conformance tarballs | SHA-256 `1f31a4535da2a7622cdbfbbeed3cda1a1a2ae6eade5ac4e1c56967bbcd11990f` |
| Release source inventory digest | `874423b9a69b7f0d665cf3a4f21de9b710bd8251b84479e98d796c251fd2f63f` |
| Release report SHA-256 | `c05c07f83f7c42f788b4a242374ca860fdd40484e164fa43c5eae215f50f0f62` |
| Conformance provenance SHA-256 | `75d48e5304eda5ecf7d28f6d568d900c127eb3725ee6337a7b476a4a52382af4`, 1,550 source inputs |
| Scalar Wasm SHA-256 | `7dce87381ffa4dd450fee236a30946c20dc82a72e8b48c40d4cfc28da002c204` |
| Threaded Wasm SHA-256 | `d46d820dcf660b165a7860d31ff0b09de54875016b0700ba295102a80a4c86bd` |
| Current release policy SHA-256 | `45d1884ae5623cc7c7d4a478b7fbd642ace730b98c362bdc8a973386cb5c965f` |

[Run 38625993733](https://github.com/mia-cx/ditherette/actions/runs/38625993733)
finishes with overall conclusion **failure**. Its release validation and website deployment jobs succeed.
The npm job fails at size review. Registry checking, trusted publication, and npm tagging skip.
The website deployment also completes its website tag/release step; that does not publish npm.

The [publication artifact](https://github.com/mia-cx/ditherette/actions/runs/38625993733/artifacts/11925226374)
contains the exact tarball and `release-report.json`.
The [conformance artifact](https://github.com/mia-cx/ditherette/actions/runs/38625993733/artifacts/11925075990)
contains its tarball, source/package/tool/oracle inventory, and completed browser logs.
Their GitHub archive digests are respectively
`821760250fa3df42d912a3dbf0c02db2fb53b988c915d15150b4521d66a74122` and
`abea0db572ac23a53264e888f0b3d69c0a94002059dc99dc522d812211c54232`.
These archive digests identify GitHub's artifact containers, not the npm tarball.

Local verification independently hashes both tarballs and matches all 54 package files
against conformance provenance. It checks every packed path, file digest, and raw/gzip/Brotli size.
The complete archive sizes also match the release report.
The installed-browser drivers perform offline tarball installs before exercising fixtures.
These bindings support reusing the following results for this exact artifact:

| Current check | Completed evidence | Practical limit |
| --- | --- | --- |
| Native scalar/threaded, types/interface, private ownership, staging, generated glue | Release validation job steps succeed; interface 83 tests, staging 2, glue 4, private ownership 27 | Source-bound suites, not performance evidence |
| Installed scalar conformance | 17 passed, zero failed/skipped; Chromium, Firefox, WebKit | Original resize/matching/field/diffusion/Process fixtures; not every S41 workload |
| Installed threaded lifecycle | 12 passed, zero failed/skipped; Chromium and Firefox | WebKit explicitly excluded |
| Actual automatic policies | 23 passed, zero failed/skipped; Chromium and Firefox | Behavior checks do not certify performance thresholds |
| Website validation | Types/tests and production build/e2e steps succeed; 287 unit tests in 40 files | Website delivery is separate from npm qualification |

Scalar logs record the same memory observations in all three engines.
Initial Wasm memory is 1,245,184 bytes; steady and both repeated batches are 1,310,720 bytes.
The maximum-axis high-water observation is 4,194,304 bytes with an 8,388,608-byte capacity limit.
Disposal leaves 1,310,720 Wasm bytes and preserves durable outputs.
The 262,144-byte processing-capacity limit is not a total browser-memory limit.
These results establish bounded fixture reuse and preflight, not maximum-area feasibility.

`e1df9fdc..c16ba62a` changes workflow/release tests, release docs, and `DIRECTION.md`.
Package source, core source, manifests/lockfile, and frozen checkpoint do not change in that interval.
The original build source remains `e1df9fdc`; source-bound release verification must use that checkout.
Later merged runtime or dependency work requires a fresh delivery artifact and a new inventory.

## Tool, browser, and oracle inventory

| Tool | Current artifact identity |
| --- | --- |
| Node / pnpm / npm | `24.19.0` / `11.13.1` / `11.17.0` |
| Scalar Rust / Cargo | Rust `1.97.0`, commit `2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`; Cargo `1.97.0` |
| Threaded toolchain | `nightly-2024-08-02`, rustc `1.82.0-nightly`, commit `8e86c9567154dc5a9ada15ab196d23eae2bd7d89` |
| TypeScript / wasm-pack | `6.0.3` / `0.15.0` |
| wasm-bindgen / wasm-opt | `0.2.121` / `117` |
| Playwright | Package-pinned `1.59.1`; executable/tool digests remain in conformance provenance |
| Chromium / Firefox / WebKit in CI | `147.0.7727.15` / `148.0.2` / `26.4` |

This research worktree has no `node_modules`.
Its command-line Node, pnpm, and rustc match the versions above.
`wasm-pack`, `wasm-bindgen`, and `wasm-opt` are absent from its shell PATH.
That is a local preparation prerequisite, not a failure of the completed CI build.

The [frozen checkpoint](../tools/spec-freeze/checkpoint.json) retains the original identity
`cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, content SHA-256
`17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Its contract amendment remains `e0101af71410e7551d8de7ed4d54056e3d7f0fd8`, content SHA-256
`a5f90b9b3b864c4826748646818bbe1c903de958811f4c5af3345082cf5ad47d`.
The current checkpoint includes 13 versioned extensions, including coverage resize, effects,
curves, masks, dither modes, and palette fit. Its file SHA-256 is
`f4d1f8738a56fffe5c61848d5f76a90f5f5dbed23619374ec12584d47b8facd2`.
The guard file SHA-256 is `9007cb16cdf203d6cf0706d37dbff8a74ac1f3757af2aac10dfdec52c72d8edc`.
Both match recorded conformance source inputs. Original v1 resize/image source matches S43;
the guard has since changed to support extensions and syntax tooling.

Conformance provenance records these oracle artifacts:

| Oracle file | Bytes | SHA-256 |
| --- | ---: | --- |
| `ditherette_bench_oracle_bg.wasm` | 788,579 | `26630a7473cea86fc2538d689d0d276a52f1c4c5ee2323dd18fe1623e7d2f1fd` |
| `ditherette_bench_oracle.js` | 8,141 | `c9c1af397fdf88fd3c2f5ea4e542541620cde4f9360ebca27242aafeaf2b606b` |
| `manifest.json` | 131,275 | `e1ecf666f7d3064c4d0d00dc123ef3925b2e3717e7d88c881511fb6f9d3ede1e` |

The CI upload retains oracle inventories, not these compiled oracle files or the generated Yliluoma fixture file.
Their identities are recorded build evidence; this task does not independently rehash their missing bytes.
Fresh workload verification must prepare and retain its own target-local oracle and fixture identities.

## Retained blocker reconciliation

“Needs new check” preserves the gate while refusing to relabel an old result as current.
Historical sample counts and failures stay attached to their original sources and artifacts.

| Retained qualification item | Current classification | Evidence or next action |
| --- | --- | --- |
| Integrated package conformance and earlier fixture-import repair | Resolved for the recorded fixtures/artifact | Same tarball digest and completed scalar/threaded/automatic logs above |
| Scalar reuse memory, input ownership, outputs after disposal | Resolved for the recorded fixtures/artifact | Current scalar memory logs, ownership tests, Chromium/Firefox lifecycle fixtures |
| Seven historical cold regressions | Needs new check; gate retained | [Anchors](83-anchors.md) identify cold Lanczos3 in native/C/F/W and cold Process in C/F/W. Fresh pre-S32/current comparisons required |
| Nine actual-TypeScript regressions | Needs new check; gate retained | [Original matrix](83-matrix.md) covers three admitted operations across C/F/W. Current tarball differs; frozen spec cannot substitute for actual JS |
| Two Firefox warm automatic regressions | Needs new check; gate retained | [Continuation](83-automatic-continuation.md) records cases 8/9 at ratios 1.227/1.288. These are historical, not current ratios |
| 21 missing scalar release cells | Needs new check; coverage incomplete | Firefox 9–14 and WebKit 0–14; interrupted Firefox 9 requires both new alternating pairs |
| Four noisy anchors | Needs new check | Native warm quantize, Chromium cold quantize, WebKit cold quantize, WebKit warm Lanczos3. Retain timer-resolution and pair-spread gates |
| Three scalar compiled-startup controls | Needs new check | [Original matrix](83-matrix.md#initialization) is inconclusive; rerun `initialization` case 1 across C/F/W |
| Candidate WebKit resize and automatic Chromium warm mixing noise | Needs new check where still required | Rejected batching candidate stays unselected. Current-policy automatic case 9 needs independent paired evidence |
| Bilinear frozen differences | Needs new check; approval remains blocked | Historical release case 4 and automatic case 2 differ from frozen v1. Exact scalar/thread agreement does not clear them |
| General website field/diffusion/perceptual/mixing equivalence | Still unavailable outside admitted fixtures | Retired TS/default activation is complete; missing semantic equivalence remains a qualification limitation |
| Capped Playwright pipe failure | Needs full-size check; code repair already present | `b1a86c75` moves bulk JSON to bounded HTTP. [Current transport](../scripts/benchmark-public-browser.mjs) returns only an acknowledgement through Playwright |
| Capped 8192×8192 feasibility, native decoder, memory and shutdown | Still blocked on the untimed feasibility result | Existing hex envelope and HTTP fixtures do not prove full-size allocations or frozen serializer feasibility |
| Capped performance matrix | Still blocked | Starts only after successful full-size untimed feasibility and parent clearance |
| Optional-thread WebKit disposal and host termination | Still blocked | Current CI excludes it; fixed runnable engine and actual lifecycle proof remain missing |
| Initial RC size acceptance | Resolved | [Merged PR #217](https://github.com/mia-cx/ditherette/pull/217) approves the baseline; that acceptance remains valid |
| Growth since accepted RC baseline | Still blocked | 74 current size findings below; no budget update or exemption |
| Website activation and TypeScript retirement | Resolved as implementation/activation | #359 and [current direction](../DIRECTION.md) state these shipped; they do not certify release performance |
| npm publisher/environment readiness | Needs new check | `npm-publish` exists with no protection rules or branch policy. OIDC permissions are configured, but registry/publication steps skip; trusted-publisher mapping is unverified |
| Stable-release hold | Still blocked | Policy retains `S41 release gates, including retained WebKit threaded cleanup, remain unresolved.` |

The historical “18 regressions” means 7 + 9 + 2 cells on the old artifacts.
It is not a count of current failures. Current conformance also cannot turn those cells into passes.
The old “all implementation PRs unmerged” and website activation holds describe historical state.

Original identities remain:

| Historical record | Source / artifact |
| --- | --- |
| Pre-S32 accepted anchor | Source `d51a70a2daf054357d16ea66b235da3733c02888`; tarball `83a07b5a9bc7f0f04ba1bcb4c6db13f8daa6101bf4b656651aa06be9f1cf59eb` |
| S41 selected package | Source `a895267baea624a6e89bfcef6c5147f170e8a8f7`; tarball `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d` |
| S41 continuation | Source `f0736c782615a911652ecaf21d1f9a8660ae3e3d`; unchanged S41 tarball |
| S43 | Build source `c43cea1269fcd666835d41c07d82a1c451604107`; fixture revision `f331ffcfde354726d8f1266e3839dbc6bffef1e0`; tarball `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90` |

The [S41 report](83-release-status.md) SHA-256 is
`0680326cdec151e6d5d7d5a8dd9e3c0a50ec87d78f3d3d65a88489e1b22badf7`.
The [S43 report](85-release-readiness.md) SHA-256 is
`d11258cbc05036f215f318cdf8700b6017d343c508e31e9fdd0eff542bfdda0b`.
No historical report changes here.

## Current size growth and payload attribution

The unchanged policy requires review for every new item and every raw/gzip/Brotli increase above 10%.
Recomputing `sizeReview` produces exactly 74 findings, including 12 new path items.

| Item | Approved raw / gzip / Brotli | Current raw / gzip / Brotli | Growth, raw / gzip / Brotli |
| --- | --- | --- | --- |
| Complete tar archive | 1,358,336 / 467,608 / 315,843 | 2,207,744 / 767,528 / 502,554 | 62.53% / 64.14% / 59.12% |
| Scalar Wasm | 447,703 / 195,187 / 155,981 | 794,237 / 328,521 / 250,397 | 77.40% / 68.31% / 60.53% |
| Threaded Wasm | 625,739 / 230,877 / 181,904 | 1,023,063 / 371,425 / 280,856 | 63.50% / 60.88% / 54.40% |

The following table accounts for all 74 findings. `raw`, `gzip`, and `br` each count as one finding.
`VARIANT` expands to both `scalar` and `threads`; comma-separated names each identify a file.

| Paths relative to package | Failed metrics / count | Attribution |
| --- | --- | --- |
| `README.md` | raw, gzip, br / 3 | Expanded public API documentation |
| `dist/compiled-effects.js`, `.d.ts`; `dist/validation-effects.js`, `.d.ts` | new / 4 | Added compiled-effects and validation API |
| `dist/index.d.ts` | raw, gzip, br / 3 | Added public types/exports |
| `dist/scalar.d.ts` | raw, br / 2 | Added processor methods |
| `dist/scalar.js` | raw, gzip, br / 3 | Added effect/analysis/compilation boundary code |
| `dist/types.d.ts` | raw, gzip, br / 3 | Effects, v2 recipes, curves, masks, palette-fit types |
| `dist/validation-fields.d.ts`, `dist/validation-fields.js` | gzip, br each / 4 | Expanded field/dither contracts |
| `dist/validation-process.d.ts`, `dist/validation-process.js`, `dist/validation.d.ts` | raw, gzip, br each / 9 | Expanded recipe/options contracts |
| `dist/wasm/VARIANT/README.md` | raw, gzip, br each / 6 | Expanded crate documentation |
| `dist/wasm/VARIANT/ditherette_wasm.d.ts` | raw, gzip, br each / 6 | Added Wasm methods/types |
| `dist/wasm/VARIANT/ditherette_wasm.factory.js` | raw each / 2 | Larger instance-owned generated bindings |
| `dist/wasm/VARIANT/ditherette_wasm.js` | raw, gzip, br each / 6 | Larger generated normal bindings; duplicated-code review candidate |
| `dist/wasm/VARIANT/ditherette_wasm_bg.wasm` | raw, gzip, br each / 6 | Main real growth; feature contributions need section attribution |
| `dist/wasm/VARIANT/ditherette_wasm_bg.wasm.d.ts` | raw, gzip, br each / 6 | Added Wasm exports |
| `dist/wasm/VARIANT/snippets/ditherette-wasm-e853852a04d98cf8/src/wasm/{copy,progress,quantize}_helpers.js` | new each / 6 | Hashed-path churn; replacements for the old snippet prefix |
| `dist/wasm/VARIANT/snippets/ditherette-wasm-e853852a04d98cf8/src/wasm/effects_helpers.js` | new each / 2 | Added effects helper |
| `$tarball` | raw, gzip, br / 3 | Complete-archive growth |

Six “new” helper findings are path replacements from `ditherette-wasm-7b3366660ebe4c12`.
The old paths are absent from the current tarball, so the artifact does not contain both generations.
Normalized helper sizes remain within 10%: copy 6,175→6,210 raw, 1,609→1,650 gzip;
progress 787→809 raw, 436→444 gzip; quantize 1,288 raw and 585 gzip unchanged.
The six added files are four root effects files plus two 173-byte effects helpers.
Hash normalization explains the findings; it does not change the existing checker or approval requirement.

Added features have source/spec references in the frozen extensions and public package code.
They include effects, compiled lookup/mask APIs, v2 coverage resizing, curves, dither modes,
and palette-fit/recolour analysis. Their presence explains expected growth categories,
but this inventory does not assign binary bytes to individual features.
The ordinary build does not enable `gpu` or `bench-subjects`; the threaded build adds `threads`.
Attribute the Wasm growth before blaming the optional wgpu dependency.

The raw installed payload grows by 846,209 bytes. The two Wasm files account for 743,858 bytes, or 87.90%.
They occupy 83.95% of the current raw installed payload.
Potential avoidable payload includes both normal JS bindings alongside their factories, 71,758 raw bytes,
and repeated generated documentation/metadata. Normal bindings already existed in the baseline;
their entire size is not newly introduced growth. Factory declarations refer to normal binding types,
so pruning requires an import/type-use audit and installed-tarball validation.
The current file contract rejects source, maps, tests, benchmarks, and build markers; no such debris appears.

## WebKit fix availability and cleanup procedure

[WebKit fix `03e836de2f7bd5627a95f60357633d59fb6bb18d`](https://github.com/WebKit/WebKit/commit/03e836de2f7bd5627a95f60357633d59fb6bb18d)
lands on 2026-08-20 at `319508@main`. It handles worker termination while JavaScript/Wasm atomic waits are parked.
An engine version string alone does not establish its inclusion.

| Engine candidate | Source provenance and availability | Fix inclusion |
| --- | --- | --- |
| Pinned Playwright 1.59.1, WebKit 2272 / 26.4 | [Upstream config](https://github.com/microsoft/playwright/blob/v1.59.1/browser_patches/webkit/UPSTREAM_CONFIG.sh) pins `482d54b279fd6147a45aaa122242f2ac315f46df`; cached GTK bundle built 2026-03-23 | Upstream base is 15,425 commits behind the fix |
| Cached Playwright 1.61.0, WebKit 2311 / 26.5 | [Upstream config](https://github.com/microsoft/playwright/blob/v1.61.0/browser_patches/webkit/UPSTREAM_CONFIG.sh) pins `d8fa5ad85d476be6d60c96ace769a2c9293bc2a8`; bundle built 2026-06-10 | Upstream base is 7,556 commits behind the fix |
| Latest stable Playwright 1.64.1, WebKit 2370 / 27.2 | [Upstream config](https://github.com/microsoft/playwright/blob/v1.64.1/browser_patches/webkit/UPSTREAM_CONFIG.sh) pins `56453fdfe0b0ca6258c23e6453b34f70b885d13f`, dated 2026-07-20; not locally installed | Upstream base is 1,981 commits behind the fix |
| Development source at `9d305c9bb565878ba53a9c38585cff4bf95085c9` | [Upstream config](https://github.com/microsoft/playwright/blob/9d305c9bb565878ba53a9c38585cff4bf95085c9/browser_patches/webkit/UPSTREAM_CONFIG.sh) pins `5b391e6d3b14d549908dad2c6695999291313f30`; browser catalog WebKit 2376 / 27.2 | Base includes the fix, 16 commits later; matching runnable binary not established |

GitHub compare API ancestry confirms these source relationships.
Both cached `pw_run.sh --version` attempts exit with the same error:

```text
MiniBrowser: error while loading shared libraries: libgtk-4.so.1: cannot open shared object file: No such file or directory
```

There is no verified runnable local WebKit containing the fix.
CI can run pinned scalar WebKit, but that old engine still lacks the fix.
The repository already provides `ditherette-bench-pair prepare-webkit INSTALL PRIVATE_LIBRARIES NEW_DIRECTORY`
for a task-owned WPE launcher. Reuse that mechanism with a verified newer engine and its dependencies.
Keep the package lockfile and shared browser caches unchanged.

After the parent assigns an untimed test window, the runtime owner must:

1. Obtain a binary tied to an upstream revision containing the fix, or a documented backport.
   Retain the download/build identity, executable/dependency hashes, and observed browser version.
2. Run the independent atomic-wait diagnostic against the recorded release tarball.
   It must observe one held worker lock, terminate it, and observe zero locks.
3. Run ownership, partial-startup failure, and host-termination fixtures without changing assertions.
   Disposal must release only the disposed pool, then release the remaining pool.
   Host termination must release its actual worker tree during startup and processing.
4. Retain zero-failure/zero-skip logs, exact tarball and engine identities, successful process exit,
   and owned-process cleanup. A diagnostic pass alone cannot clear the package lifecycle gate.

These commands are prepared instructions, not completed checks. They require installed frozen dependencies
and a task-owned compatible launcher set in `DITHERETTE_TEST_WEBKIT_EXECUTABLE`:

```sh
releaseScratch=/home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/release-evidence
mkdir -p "$releaseScratch/tmp" "$releaseScratch/webkit-checks"
export TMPDIR="$releaseScratch/tmp"
export DITHERETTE_TEST_TARBALL=/home/mia/.cache/ditherette-tmp/npm-release-2026-10-14/artifact/release/ditherette.tgz
export DITHERETTE_TEST_TARBALL_SHA256=1f31a4535da2a7622cdbfbbeed3cda1a1a2ae6eade5ac4e1c56967bbcd11990f
export DITHERETTE_TEST_ENGINES=webkit
: "${DITHERETTE_TEST_WEBKIT_EXECUTABLE:?Set the verified fixed-engine launcher}"
flock /home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/build.lock \
  node --test --test-concurrency=1 packages/ditherette/tests/thread-atomic-wait.diagnostic.mjs \
  > "$releaseScratch/webkit-checks/atomic-wait.log" 2>&1
# Continue only after the independent diagnostic exits successfully.
flock /home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/build.lock \
  node --test --test-concurrency=1 packages/ditherette/tests/thread-observer-browser.test.mjs \
  packages/ditherette/tests/threads-ownership-browser.test.mjs \
  packages/ditherette/tests/threads-failure-browser.test.mjs \
  packages/ditherette/tests/threads-host-browser.test.mjs \
  > "$releaseScratch/webkit-checks/lifecycle.log" 2>&1
```

## Bounded next tasks

| Task boundary | Files or evidence owned | Completion criterion |
| --- | --- | --- |
| Source/oracle drift audit | New current evidence; narrow v1 bilinear diagnostics and versioned recipe fixtures | Reconstruct release case 4 and automatic case 2; retain native/Wasm reference and actual bytes. Original frozen files stay unchanged; unresolved visual changes go to Mia |
| Performance coverage | Fixed generator/prepared manifests and fresh paired reports | Complete the ranked queue below with recorded case/source/artifact/tool/browser identities. Coordinate #360 separately; all missing/noisy/incorrect cells remain explicit |
| Capped feasibility | Benchmark transport/probe only, after checking existing HTTP repair | One untimed full-size 1×1→8192×8192 Process run finishes frozen/native reference, Process/staged equality, exact hex transfer, Rust decode, memory evidence, and shutdown |
| Ownership/memory and WebKit | Installed artifact fixtures and task-owned engine evidence | Reuse current fixture results; cover added effects/analysis/compiled APIs where browser coverage is absent, then prove fixed-WebKit disposal and host termination |
| Size attribution | Section/symbol/import evidence and a proposed package-only repair | Separate feature growth, renamed paths, and removable duplicate payload. Validate exact rebuilt tarball; leave budget/hold approval with the parent/Mia |

The capped feasibility task retains the actual indexed palette, transparency, warnings,
and exact 67,108,864 index bytes. It keeps default package memory limits and the explicit
384 MiB benchmark retention override. It records RSS/allocation limits and shutdown;
the old 2,027,572 KiB peak is a historical individual-child observation, not current aggregate memory.
Frozen oracle serialization still expands arrays before compacting them, so HTTP alone does not prove feasibility.
No committed end-to-end untimed maximum-area probe command currently establishes this gate.
The probe owner must supply that command before scheduling the full-size check.

## Ranked measurement queue after PNG

Every rank waits for PNG completion and a parent-granted exclusive window.
Preparation and untimed conformance finish first. Drain builds, tests, and implementation agents.
Use [EXECUTION.md](../crates/ditherette-bench/EXECUTION.md) and
[PAIRED.md](../crates/ditherette-bench/PAIRED.md); the build lock does not replace the benchmark lease.
Run prebuilt binaries only during timing. Keep cold, warm recomputation, and identical-result hits separate.
Preserve two alternating pairs, at least five valid samples, the declared sample/wall budget,
timer-resolution checks, exact conformance, and each case's 10% regression/spread gate.

| Rank | Generator commands before the window | Measurement command during the window | Prerequisites and bound |
| --- | --- | --- | --- |
| 1 | `"$releaseGenerator" anchors-native 2 "$experimentJson" "$hostNotes"`; for public cold resize use `anchors 2`, for public cold Process use `anchors 6` with the same argument form | `DITHERETTE_BENCH_QUIET=1 "$pairCoordinator" run "$preparedJson" "$newResults"` per prepared case/runtime | Fresh pre-S32 accepted source and current delivery candidate with shared protocol-only updates. Start with cold Lanczos3, then browser cold Process. Two pairs per cell; do not launch all anchors blindly |
| 2 | `"$releaseGenerator" release 9 "$experimentJson" "$hostNotes"`, then indices 10–14 and WebKit 0–14 using new destinations | Same `run` command per case/engine | Fresh ordinary scalar assets and target-local exactness; Firefox optimization configuration and runnable scalar WebKit. Firefox 9 starts with both new pairs. Split six Firefox and fifteen WebKit cells into bounded windows, 84 workers total |
| 3 | `"$releaseGenerator" automatic 8 "$experimentJson" "$hostNotes"`; repeat for `automatic 9` and `initialization 1` with new destinations | Same `run` command per case/engine | Actual scalar/required-thread host policies for automatic C/F cases; bytes/compiled scopes remain separate. Prioritize Firefox warm cases 8/9, then Chromium case 9 and the three compiled scalar startup cells |
| 4 | `"$releaseGenerator" typescript 0 "$experimentJson" "$hostNotes"`, then 1/2; select noisy anchor indices from the fresh inventory | Same `run` command per equivalent case/runtime | Actual JS closure and admitted nearest/no-dither/sRGB fixtures. Four noisy anchor cells retain their original scopes. #360 needs a separate 2600×4168→650×1042 fixed-support Lanczos2/3 plan, Firefox plus Chromium control, and attribution fixtures before using this runner |
| 5 | `"$releaseGenerator" capped 0 "$experimentJson" "$hostNotes"` | Same `run` command, only after the untimed feasibility gate | Exact full-size feasibility, native decoder, memory, and shutdown pass first. C/F/W scalar only; two pairs each, 12 workers. The historical 60-minute estimate is provisional, not an approved new budget |

The ranked table selects work; these exact templates prepare and run a single assigned case.
Set paths to the recorded binaries and actual clean role checkouts/artifacts before use:

```sh
releaseScratch=/home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/release-evidence
mkdir -p "$releaseScratch/tmp"
: "${releaseGenerator:?Set the prebuilt release_integration_plan executable}"
: "${pairCoordinator:?Set the recorded ditherette-bench-pair executable}"
: "${hostNotes:?Record actual unrelated host load}"
: "${acceptedBench:?Set the recorded accepted ditherette-bench executable}"
: "${acceptedRevision:?Set its full clean source revision}"
: "${candidateBench:?Set the recorded candidate ditherette-bench executable}"
: "${candidateRevision:?Set its full clean source revision}"
: "${browserSources:?Set validated BrowserSources JSON for this engine and both roles}"
: "${experimentJson:?Set the assigned one-case experiment JSON}"
: "${lane:?Set the assigned lane from the ranked table}"
: "${caseIndex:?Set the assigned fixed case index}"
: "${newPrepared:?Set a new absolute preparation directory}"
: "${newResults:?Set a new absolute results directory}"
"$releaseGenerator" release inventory "$releaseScratch/release-inventory.json" "$hostNotes"
"$releaseGenerator" "$lane" "$caseIndex" "$experimentJson" "$hostNotes"
"$pairCoordinator" prepare-browser "$experimentJson" \
  "$acceptedBench" "$acceptedRevision" "$candidateBench" "$candidateRevision" \
  "$browserSources" "$newPrepared"
preparedJson="$newPrepared/pair/prepared.json"
# Parent clearance starts the quiet phase. No preparation command runs below this line.
DITHERETTE_BENCH_QUIET=1 "$pairCoordinator" run "$preparedJson" "$newResults"
```

For native anchors, use `prepare` without `browserSources`; its manifest is `NEW_DIRECTORY/prepared.json`.
`BrowserSources` comes from both ordinary `bundle-source.json` files plus a recorded runtime,
using the existing [typed schema](../crates/ditherette-bench/src/browser_assets.rs).
The required-thread role binds threaded Wasm and isolation explicitly.
Fresh artifacts must use distinct recorded role revisions; runtime-equivalent documentation revisions
can support repeatability controls but cannot show improvement over a performance acceptance anchor.

Build-only prerequisites use the shared lock, `CARGO_BUILD_JOBS=2`, task-owned temporary storage,
and new artifact directories. Existing entry points are:

```sh
flock /home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/build.lock \
  env CARGO_BUILD_JOBS=2 TMPDIR="$releaseScratch/tmp" \
  node scripts/prepare-native-benchmark.mjs "$releaseScratch/native-bundle" "$releaseScratch/native-build"
flock /home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/build.lock \
  env CARGO_BUILD_JOBS=2 TMPDIR="$releaseScratch/tmp" \
  node scripts/prepare-public-benchmark.mjs "$releaseScratch/public-bundle"
flock /home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/build.lock \
  env CARGO_BUILD_JOBS=2 TMPDIR="$releaseScratch/tmp" \
  cargo build --locked --release --manifest-path crates/ditherette-bench/Cargo.toml \
  --target-dir "$releaseScratch/plans-build" --example release_integration_plan
releaseGenerator="$releaseScratch/plans-build/release/examples/release_integration_plan"
```

Run those in each assigned clean role checkout after frozen dependency installation.
Arrange ignored compiler target directories on task-owned cache storage before preparation;
the crate build script chooses its own scalar/threaded target paths.
Build the plan example before the window and bind its executable identity.
For older Firefox, prepare a private optimizing copy with
`node scripts/prepare-firefox-benchmark.mjs FIREFOX_EXECUTABLE NEW_DIRECTORY` before snapshotting runtime assets.
No command here authorizes a budget extension, cache-policy change, or bypass of an incorrect case.

## Checks completed by this research task

- Both initial and updated rebases onto `origin/main` succeed with a clean tree.
- GitHub run/job/artifact metadata, retained logs, policy, source diffs, and upstream browser ancestry are inspected.
- Independent archive verification passes all 54 paths/digests and raw/gzip/Brotli sizes; both tarballs match.
- `node --test packages/ditherette/scripts/release.test.mjs` passes 4 tests with zero skips.
- `node --test scripts/benchmark-indexed-wire.test.mjs scripts/benchmark-public-browser.test.mjs` passes 28 tests with zero skips. These use small/fake fixtures; they collect no processing performance evidence.
- Direct contract assertions reproduce 74 size findings/12 new items, reject publication for size, and separately reject the retained hold.
- All local report links resolve, shell blocks pass `bash -n`, and the report diff passes `git diff --check`.

The browser version probes fail with the recorded dependency error and are not reported as passes.
The WebKit cleanup commands, maximum-area probe, builds, and ranked measurements remain unrun.
The parent schedules their prerequisites and windows; #359 and both publication gates remain open.
