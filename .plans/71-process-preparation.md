# S30 end-to-end process preparation

Issue #71 completes the fifth public method. Runtime, adapters, final installed
conformance, and the bounded measurement trial are complete. The implementation
retains exact staged-production behavior, including inherited resize differences.
The [measurement report](71-benchmark-results.md) owns timings and release limitations.

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
- [x] Add only the missing native process composition, following the readable frozen
  resize-then-dither sequence. Record its baseline before optimizing ownership.
  Keep intermediates in Wasm. Only the final indexed result crosses to JS.
  Preserve the RGBA8 clipping and rounding boundaries between stages.
- [x] Register the versioned public process request and private export. Reuse
  existing option types and validation. Reject the entire invalid request before
  output work, and count simultaneous prepared data, source, intermediates,
  indices, scratch, and boundary capacity. Test allocation failures and recovery.
- [x] Verify process equals the actual staged production calls for each family,
  including indices, ordered palette, transparency, warnings, and durable output.
  Retain independent frozen comparisons and any inherited resize differences.
  Do not change a landed kernel or the frozen oracle to erase those differences.
- [x] Add complete-call native/public benchmark adapters and extend the existing
  identified frozen-Wasm oracle with the frozen process composition. Declare a
  bounded, filter-specific trial before changing the baseline. Measure serially
  only after all agents, builds, and tests drain. Keep exact wins only.
- [x] Run focused integrated native, private-boundary, installed-tarball, and
  three-engine checks. Prepare the unmerged PR against the documented parent/join.
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

## Native composition baseline

`Processor::process` accepts the typed pipeline `ProcessRequest` and the existing
`QuantizeBoundary`. `process_with_allocator` exposes the existing failure-test
boundary. The implementation prepares resize and matching once, reserves source,
resized RGBA8, optional perturb RGBA8, and indices, then copies input once.
Only final indexed completion crosses the external boundary. All allocations
remain live until completion in this readable baseline. No fusion or ownership
optimization has been attempted.

The first test failed because the process module and method were absent. It now
passes against frozen native process output for all four dither families,
including full palette, transparency, warnings, and dimensions. Whole-call
failure tests and public/private adapters follow in a separate commit.
The baseline is `3335bb69acc6762a30a0b6844aef436c2e6b8de6`.

## Public adapter checkpoint

`ProcessRequest` and `RecipeV1` expose the frozen nested recipe with `match`.
The wrapper guards caller getters before normalization. Both staged validators
consume each raw property once; their normalized source view remains borrowed.
The private export shares exact resize/dither parsers and the caught indexed
sink. Existing staged resize validation order remains unchanged.

Four native process tests pass, including all seven filters, all four dither
families, one-byte-under preflight, each buffer reservation failure, allocator
overcapacity, invalid settings before allocation, and recovery. The two new
private tests cover numeric groups, all families, partial-result rejection,
reentry, disposal, and 64 repeated failure cycles with stable handles and memory.
Fresh scalar/threads builds, 33 public interface tests, and 16 private tests pass.
The public process fixture checks 423 staged compositions, all fifteen matching
modes, exact budgets, nine invalid request shapes, getter reentry, three caught
copy phases, and durable results. It also runs in the installed-browser suite.

## Installed validation and handoff

Runtime source `587339793cf70429b673e888a89d86a332541693` passes fresh package
preparation, 42 focused native tests, 33 public interface tests, and 16 private
tests. The coordinator's trusted S18 guard passes compiler, dependency, syntax,
content, and independent native/Wasm/spec/prod/threaded isolation checks.
The frozen 106-file digest remains
`17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

The installed-tarball suite passes Chromium `147.0.7727.15`, Firefox `148.0.2`,
and WebKit `26.4`. Each engine runs 423 exact actual process/staged compositions,
nine strict request failures, three caught copy phases, exact-budget recovery,
getter reentry, and durable-output checks. Existing coverage also passes 110
field fixtures, 1,650 separable compositions, 360 diffusion vectors, 367 frozen
Wasm Yliluoma vectors, and 734 actual untimed Yliluoma adapter calls per engine.

Retained preparation is under this worktree's `target/s30-validation-01/`:

| Artifact | SHA-256 |
|---|---|
| `public/ditherette.tgz` | `379c733b02bc67a24500d3ae825901d17d5fa342f93d114c20761da1aa9193b2` |
| Scalar Wasm | `170b1ad5a9a13290b0d2e07db141f35895abf3d51cb237b999b86f93122fa739` |
| Threads Wasm | `9731aad8eb95ad8aeb9dad59f13711202e3818a69b81cb14624f147f39089e71` |
| Inherited Yliluoma oracle Wasm | `8103c0f7604ca6e6c1d5d124100eec0807c32a771f0847623f5a0d0714743c3b` |

`public/build-provenance.json` binds the clean runtime revision, 1,116 tracked
inputs, 38 installed package files, build tools, and the frozen-only oracle.
`oracle-evidence/{chromium,firefox,webkit}-yiluoma-references.json` retains each
full native/Wasm output pair, oracle manifest, browser version, and tarball hash.
The seven inherited Yliluoma target differences remain at fixture indices
236, 248, 251, 254, 257, 260, and 263. No expected output or tolerance changed.

Preparation uses `node scripts/prepare-public-benchmark.mjs
target/s30-validation-01/public`. The `yliluoma_conformance` example writes
`target/s30-validation-01/yliluoma.json` using the assigned native target.
The browser command is `node --test packages/ditherette/tests/tarball-browser.test.mjs`
with `DITHERETTE_BENCH_ORACLE` pointing to `public/scripts/oracle`,
`DITHERETTE_BENCH_YLILUOMA_FIXTURES` to that JSON, and
`DITHERETTE_BENCH_ORACLE_EVIDENCE` to `oracle-evidence`, all absolute paths.
WebKit uses the retained executable at
`.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit` via
`DITHERETTE_TEST_WEBKIT_EXECUTABLE`.

That initial validation predates the joined benchmark and Process oracle.
The final evidence below supplements it without replacing its artifacts.

## Final integrated evidence

Measured source is `e5aae7bf0e1761af2f970b6da75d34cf3a813323`.
It joins benchmark/oracle checkpoint `834e882f95bb042b8356b324ef18343ef0ad1c52`
without changing runtime, private/public, spec/image, or freeze-policy bytes from
`587339793cf70429b673e888a89d86a332541693`. The immediate PR base is
`impl/v1-s30-base` at `22b6dd78a6e552596c34aa9e693ad74850426b23`.
The required merge-preserving rebase keeps all six delivered prerequisites.
Changes after measured source are documentation only; prior runtime checks remain valid.

The fresh final tarball and scalar/threads hashes equal the initial artifacts above.
The final Process oracle hash is
`300f61644c4b7757d1ad80b97c515121a5ad241fa0051e9827e448f0067ffb64`.
`target/s30-trial-01/public/build-provenance.json` binds 1,122 tracked inputs,
the package, oracle, tools, and complete artifact inventories to measured source.
The full trusted S18 guard passes that source with the unchanged frozen digest.

Final installed conformance retains 431 independently identified frozen references
and two area-attribution probes per browser. Both fresh and primed instances
produce exact Process/staged output for all eight declared pipelines in each engine.
The area case differs from frozen Process at seven indices. Its landed resize
differs in 125 RGBA bytes, each by one; frozen processing on those actual resized
bytes explains the entire indexed difference. Native/Wasm frozen references agree
for these Process fixtures. The separate blue-noise Oklab target difference remains
in diagnostics. No frozen output, tolerance, or production kernel changed.

The coordinator ran 128 serial workers and retained 2,504 samples, with maximum
live workers equal to one and all children reaped. All concrete Process/staged
paired outputs are exact. All four aggregate gates remain `Incorrect` because of
the inherited area difference. Five other case/runtime timing gates are inconclusive;
none confirms a slowdown above 10%. This is composition-overhead evidence, not
TypeScript parity, a new optimization selection, or full release acceptance.
S41 owns unresolved timing evidence and broader release comparisons.

The final runtime snapshots retain Chromium 147.0.7727.15, Firefox 148.0.2,
WebKit 26.4, Node 24.19.0, and Playwright 1.59.1. All runtime inventories,
permissions, executable digests, and aliases were revalidated before and after
copying. Firefox uses the retained S26 closure, never the live locked installation.
WebKit retains 18 hardlink aliases. `browser-sources/` records provenance and the
exact-f32 serialization receipt; complete case identities remain unchanged.

## Compiler cleanup handoff

All owned jobs are drained. The coordinator owns cleanup after PR filing.
Return `/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target`
and its `scalar` and `threads` children. The public helper also builds the independent
oracle, its API dependency, and profile checker inside that same `scalar` target.
The S30 worktree's ordinary ignored core target contains only variant links to
these assigned directories. Preserve all `target/s30-validation-01` and
`target/s30-trial-01` artifacts, snapshots, reports, and copied binaries.

## Scope limits

No rewrite of existing production kernels or shared helpers. Frozen spec/image
and freeze policy remain unchanged. No caches, progress delivery, threads,
website changes, UI work, publication, deployment, or PR merging in this slice.
S31 onward owns the later runtime work. Supplied progress callbacks retain the
current explicit unsupported behavior until S33.
