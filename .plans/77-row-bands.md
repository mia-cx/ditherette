# S35 resize and color row bands

## Scope and ancestry

Issue #77. Branch `impl/v1-s35-resize`, worktree `v1-s35-resize`.
Initial parent is S34 capability checkpoint `6296c66babdbef8967f768108988899994396f03`.
The coordinator explicitly authorizes this bounded start while S34 finishes loader error handling and cleanup.
Join its final validated head before S35 delivery or artifact preparation.
Join `92768069fe1a4cbdecdfd79795f12cd08b930a11` includes validated S34 delivery `3863cadce82b4d272a1730f3ebad828b18ac4434`.
Final S34 report-only head `d4531667e1158c2068f30614f40c9d39f8c5313e` joins through `d137603a88ac0e061c99cefc316728baffd918e1`.

Own production resize/color row adapters and policies, focused native tests, and S35 benchmark subjects.
Preserve scalar kernels, packed nearest, exact integer area, accepted fractional arithmetic, and shared convolution and mip plans.
Public color continues to use packed triplets with byte alpha. Direct quantization gains no full-image color plane.

## TODOs

- [x] Expose allocation-free area/bilinear caller-scratch row adapters. Check exact split output and reject insufficient scratch before writes.
- [x] Extend shared convolution with caller-owned support-range scratch, including overlapping support across bands.
- [x] Add complete worker/support capacity preflight and disjoint execution through existing tiling models.
- [x] Integrate those adapters into private complete-call preparation with budget-based scalar fallback.
- [x] Extend benchmark subjects for the budgeted path and retain caller-thread progress.
- [x] Join final S34 report-only PR head.
- [x] Validate actual installed scalar/threaded calls before exclusive crossover evidence.
- [x] Prepare the unmerged PR after the coordinator joins measured domain policies and validates installed artifacts.
- [x] Separate automatic per-request scheduling from per-stage developer overrides, capped by the actual pool.
- [x] Select resize policies from Chromium trial02 and Firefox trial03 evidence, preserving scalar outside measured support.

## Test seams and evidence

The first confirmed seam is the existing production plan-plus-output row adapter, extended with caller-owned scratch.
Compare split execution exactly with the landed full-call kernel, including identity axes, integer scales, and fractional support.
Budget tests use the existing fallible `CapacityBudget` and check output preservation on rejection.
The next seam is the production budgeted executor, not a test-only scheduling implementation.

No policy threshold or threaded default is chosen during this checkpoint.
Callbacks remain on the caller. Diffusion remains scalar. Trilinear retains one shared mip chain.
S34 requires a blocking-wait-capable processing host for threads. Its WebKit cleanup limitation remains a recorded release gate.

Only worktree-local `target/compiler` belongs to this task. No symlinked targets, Wasm builds, or measurements are authorized.
Drain every owned job when the coordinator requests the S34 quiet phase.

## First bounded checkpoint

Area and bilinear reuse their existing absolute-output row loops and fast paths.
The new adapters accept already-reserved f32 slices. They reject undersized scratch before changing output.
No current public dispatch selects these new adapters yet. Full worker-capacity planning remains pending.

Each new adapter fixture first fails to compile because the required caller-scratch entrypoint is absent.
After implementation, two new tests pass with physical allocation counters around execution.
They compare exact landed scalar bytes across integer scales, fractional scales, identity axes, and three representative anchors.
Area tests use band heights 1, 2, 4, and the complete output height. Bilinear tests include an uneven final band.
Caller scratch is reserved through `CapacityBudget`; every tested execution allocates zero heap blocks.
Short scratch returns `MemoryLimit` and preserves sentinel output bytes.

Focused native validation passes 15 tests across `prod_resize_row_scratch`, `prod_resize_area`, `prod_resize_bilinear`,
`prod_resize_preparation`, `prod_process`, and `prod_progress`.
Command uses `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --target-dir target/compiler` with those six test targets.
No Wasm builds, measurements, threshold selection, frozen changes, or color-plane changes occur in this checkpoint.

## Shared convolution checkpoint

The existing x-then-y row kernel accepts caller-owned f64 scratch. Its source-support calculation also supplies preflight lengths.
Bicubic and Lanczos expose that shared adapter; their scalar dispatch and arithmetic remain unchanged.
The new fixture first fails on absent row-scratch APIs. It then passes bicubic/Lanczos2/Lanczos3, both support policies,
three anchors, identity axes, direct filtering, and large x-then-y downscales with band heights 1, 3, and full output.
It proves overlapping bands need more aggregate support scratch than one full-call owner.
Physical allocation counters stay unchanged during execution; short scratch leaves output sentinels intact.

## Budgeted executor checkpoint

`RowBandBuffers<T>` reserves the entire assignment metadata and maximum scratch needed by each active worker.
The enclosing call must also charge borrowed source, kernel plans, and final output. Public integration is still pending.
The builder checks the complete requirement before allocation and checks each actual vector capacity through `CapacityBudget`.
It shares existing band generation and balanced assignment logic; direct frozen comparisons cover worker caps and order.

`execute_row_band_work` splits mutable output slices along existing assignments and uses recursive `rayon::join` when compiled with threads.
The initialized pool is a prerequisite. Scalar builds execute the same assignments without threads.
Each batch processes at most one band per worker. Progress executes on the caller only after all workers join.
A rendering failure joins the whole batch before returning. A progress failure prevents the next batch.
No default selection or threshold is added.

The complete convolution fixture checks native scalar and Rayon output equality, overlap capacity, and allocation-free worker kernels.
Its one-byte-short complete budget fails before any allocation. Every metadata/scratch reservation also has an injected failure check.
Current checks pass 26 scalar tests and 20 threaded tests, including the direct frozen-assignment fixture.
No actual Wasm or complete public-call performance claim follows from these native checks.

## Complete-call integration checkpoint

Private execution candidates live in the shared instance store. The normal policy stays scalar.
`PreparedResize` retains either scalar scratch or complete worker buffers, including metadata and overlapping support.
Preparation chooses worker buffers within the same full-call budget before output allocation. A budget miss keeps scalar.
Warm changed-source calls reuse plans and worker capacity. Pressure releases idle scratch before evicting preparation entries.
Nearest retains the landed whole-call scalar dispatch. Trilinear always retains the shared-chain implementation.

The benchmark-only Wasm control is `privateExecutionPolicy(stage, height, active_workers, pool_size)`.
Stages 0, 1, and 2 select resize, indexed, and mixing respectively; height zero clears the candidate.
The benchmark host must initialize a blocking-capable worker pool before choosing row bands.
No public recipe fields, automatic thresholds, or main-JS threading changes were added.

Native scalar and threaded library suites pass 35 tests each.
The complete-call fixture checks all seven filters, worker counts 1/2/4, exact scalar bytes,
caller-thread progress, failed callback publication, warm changed-source reuse, and scalar-budget fallback.
The capacity-charge fixture confirms complete preparation releases idle scratch before LRU eviction.

## Benchmark candidate handoff

Nine `candidate:resize:<filter>:complete-call` subjects cover nearest, area, bilinear, and both convolution support policies.
They use the actual private Processor call with cold application state, source snapshot, hashing, full preparation, and output copy.
Optional existing `tile_policy` height/count parameters select bounded bands. The same subject without those parameters remains scalar.
These native adapters do not claim installed-browser boundary or warm-cache evidence.

The benchmark-only `privateExecutionPolicy` control preserves other stage selections.
Sequential stage 0 and stage 1 calls can therefore configure combined resize/indexed candidates.
A complete-process fixture sets both; rerun it after the coordinator joins S36 to exercise both accelerated stages.

Focused native threaded validation passes 37 library tests plus 31 integration tests.
Integration targets cover budgeted benchmark registration, complete process, progress, convolution reference equality,
caller scratch, fallible preparation, and tiling. Scalar and threaded complete-call results are exact in all nine benchmark subjects.
Only `target/compiler` was used. No Wasm builds or benchmark measurements ran.
The coordinator owns artifact builds, exclusive host-worker crossover evidence, policy acceptance, and PR handoff.

## Measured-policy delivery

Resume on `delivery/v1-s35-resize` in `v1-s35-delivery`, based on `2bd25aeddb3fb999c8daab4c516855f8f961233d`.
The coordinator has ended the measurement quiet phase and confirmed all 200 Firefox trial03 workers exited.
Only this worktree's `target/compiler` may compile native checks. Wasm builds, measurements, PRs, and pushes remain coordinator-held.

Each domain supplies a measured request candidate to `Store::row_policy(stage, measured)`.
Explicit scalar is distinct from absent override. The private Wasm setter changes one stage only.
Override presence stays outside semantic cache keys. Threaded policies cap the actual initialized Rayon pool.
Normal scalar builds remain scalar; native developer fixtures may still force sequential row-band execution.
Automatic domains use `execution::worker_budget()` to choose measured worker counts. The resolver rejects oversized
automatic requests instead of inventing an unmeasured count; explicit overrides retain actual-capacity clamping.
Native library checks pass 37 tests with threads and 37 without, including isolated two-worker-pool resolution,
per-stage preservation, explicit scalar, complete-call equality, callback recovery, and memory-pressure fallback.

### Selected resize evidence

The immutable combined measurement source is `5d16c5682f354fecd75ca7f761802d9e2ea75ab5`.
Reports are under `v1-s35-37-bench/target/rows-trial-02/chromium-results/report.json` and
`v1-s35-37-bench/target/rows-trial-03/firefox-results/report.json`.
The coordinator completed both 50-case, two-pair trials and reaped all workers before implementation resumed.
Firefox trial03 uses a hash-bound, update-disabled browser configuration after the updater mutated trial02's runtime.
This runtime correction changes no processing kernels or package artifact.

Cold candidate/scalar median ratios from complete host-worker calls:

| Measured class | Workers / rows | Chromium | Firefox | Samples per accepted/candidate role in each pair |
|---|---|---|---|---|
| Area 1537×1025 → 769×513 | 2 / 32 | 0.9163 | 0.8484 | 20/20 in both engines |
| Area, same shape | 4 / 128 | 0.9027 | 0.8152 | 20/20 in both engines |
| Center bilinear, same shape | 2 / 32 | 0.9152 | 0.8495 | 20/20 in both engines |
| Center bilinear, same shape | 4 / 128 | 0.9004 | 0.8292 | 20/20 in both engines |
| Center scale-aware Lanczos3 2048×1536 → 512×384 | 2 / 32 | 0.8522 | 0.7448 | Chromium 20/20; Firefox 10/13 |
| Lanczos3, same shape | 4 / 128 | 0.7387 | 0.6287 | Chromium 20/20; Firefox 10/15 |

Firefox Lanczos3 reaches the declared 10-second sampling cap. Its actual counts satisfy the existing minimum-five protocol;
the coordinator explicitly accepts those results without inventing a mandatory-20 gate.
Final-image-hit four-worker ratios remain within the regression gate: area 0.9946/0.9950, bilinear 0.9916/1.0009,
Lanczos3 1.0011/1.0156 (Chromium/Firefox). Every final-hit role records 20 samples.

`paired.rs` emits each production comparison followed by a reference probe. Production records 0 and 2 show exact
same-artifact scalar/row bytes and metadata for all selected cases. Bilinear's reference probes retain 33 one-byte
frozen differences in 1,577,988 output bytes. Its gate remains `incorrect`; row scheduling introduces no new approximation.
S41 must preserve this inherited discrepancy and its approval status. Do not classify reference probes as production-pair drift.

Automatic area/center-bilinear scheduling requires each source axis to equal twice its output axis minus one,
with output width ≥769 and height ≥513. Automatic center/scale-aware Lanczos3 requires exact fourfold downscaling,
with output width ≥512 and height ≥384. These conservative lower bounds retain the measured scale classes.
The initialized pool selects four workers/128 rows when capacity is at least four, two/32 when capacity is two or three,
and scalar otherwise. Explicit developer overrides still force a selected path within actual capacity and memory limits.
Nearest, small inputs, other filters, other anchors/support, identity, upscale, and other scale classes stay scalar.

The focused automatic complete-call fixture compares all three selected filters with forced scalar output.
It verifies caller-thread callbacks, failed-publication recovery, warm changed-source preparation reuse, and bounded capacity.
Policy fixtures cover both size boundaries, pool capacities one through eight, and rejected filter/scale/anchor classes.

Native validation for this delivery passes 40 threaded library tests, 39 scalar library tests, and 27 focused
threaded/benchmark integration tests. Commands use `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml
--target-dir target/compiler`, with `--lib` (scalar and `--features threads`) and `--features threads,bench-subjects`
for `bench_resize_budgeted`, `prod_process`, `prod_progress`, `prod_resize_row_scratch`, `prod_resize_preparation`, and `prod_tiling`.
The S35-only branch still warns that S36's working-capacity charge/release helpers lack consumers before its integration join.
No frozen spec, image storage, landed resize kernels, Wasm artifacts, measurements, PRs, or remote branches change here.
The coordinator owns the final installed-artifact validation and unmerged PR handoff.

## Final delivery disposition

Publication branch `delivery/v1-s35-resize` rebases with merge ancestry preserved onto published S34
`d4531667e1158c2068f30614f40c9d39f8c5313e`. Its diff contains resize and shared execution/benchmark support;
S36 fields and S37 mixing remain separate stacked PRs. All implementation TODOs are complete.

The coordinator's ordinary combined package from `dc81818a` passes the trusted frozen guard.
Tarball SHA-256 is `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
The installed automatic-policy fixture passes 23 checks, including nine cases per Chromium/Firefox engine.
Exact invocation and cleanup evidence live in `.plans/77-79-auto-host.md`. These untimed checks confirm correctness,
not a second performance measurement. Earlier native subset results above remain the S35 branch validation.

The completed paired reports contain 400 serial workers, 7,215 samples, and 200 exact production comparisons across both engines.
Their paths are recorded under Selected resize evidence. Reference probes retain inherited bilinear frozen drift.
S41 owns that discrepancy's approval status, inconclusive controls, warm-policy follow-up, and the pinned WebKit threaded cleanup gate.
No frozen expectations, tolerances, production kernels, or measured artifacts change during PR preparation.
Delivery checks pass `git diff --check` and 18 focused Node policy, stage-cache, and artifact-preparation tests.
Resize, shared execution, Wasm wiring, and package build inputs match `dc81818a`; remaining production differences
are the separate S36/S37 families and S36's test/developer-only getter annotation.
