# S28 literal native diffusion baseline

Issue #69. The literal baseline precedes benchmark registration, bounded scratch, and public integration.

## Prerequisites

Start from S26 public baseline `089251287e387cb575e22e8993d8989a371a089d`.
The prerequisite join is `eb80d780249b2728e0b14dcb5904c870286f71f5`.
It includes validated S25 PR #114 head `3a9db011207a44f230ae519edc93c900a747c021` and the S26 baseline above.
Duplicate-history conflicts retain S26's packed forward adapter and shared private quantize parser extraction.
Both conflicted files match the S26 parent after resolution. The selected S25 matcher remains unchanged.
The join changes no production or package processing bytes from S26; only inherited documentation and benchmark evidence are added.

The prerequisite join passes 13 scoped field/processor native tests and wasm32 cargo check using the assigned S24 Wasm target.
The final S25 join includes its benchmark-module formatting correction.

## TODOs

- [x] Inspect existing production helpers, resolve and validate the prerequisite join, and record ownership.
- [x] Copy frozen diffusion and missing coordinate helpers, with explicit mechanical constructor/import substitutions.
- [x] Compare all four kernels, both feedback modes, all fifteen metrics, scan orders, alpha policies, placement, failures, and complete metadata against frozen references; commit the literal baseline.
- [x] Join accepted S26 `3915f60519995cb9087a18b3bfd6bd7220ae804a` and register the full-image native benchmark before optimization.
- [x] Implement fallible three-row scratch with exact arithmetic and width-based capacity checks.
- [x] Expose all four diffusion kernels through the private processor and public package; verify public calls and memory failures.
- [x] Prepare candidate benchmark subjects for the coordinator's exclusive comparisons.

## Copy boundary

No existing production diffusion kernel, error ring, or generic coordinate-dither helper was found.
The frozen module supplies all taps, full-image f32 work, finite checks, scan order, sinks, and both feedback recipes.
Existing production palette preparation, matcher preparation, metric functions, packed conversion, and placement already cover those semantics.
The two frozen constructor calls use private baseline helpers invoking those landed preparation APIs with an unbounded u64 budget.
That adapter is baseline-only, not a public memory guarantee. Full-image allocations remain the literal reference behavior in this phase.
Missing generic coordinate helpers are copied verbatim into a private diffusion common module.
Its squared-distance import reuses the landed `prod::quantize::metric::euclidean3_squared` with identical arithmetic.
Neither palette/matcher implementations nor landed color, resize, spec, image, or freeze policy files change.

## Baseline validation

Native `prod_diffusion` passes six tests. Its complete-result matrix covers 8,640 combinations of four kernels, two feedback modes, fifteen matching policies, both scan orders, three alpha policies, three placement settings, and four image shapes.
Another 240 combinations match frozen direct quantization at zero strength.
Independent witnesses cover byte-feedback rounding, first ties, and each kernel's raster/serpentine result.
The remaining comparisons cover structured arithmetic/validation failures, transparent sinks and fallback, transparent-only/truncated palette warnings, legacy coordinate taps, padded rows, and three/four-channel input.
The private constructor-adapter unit test checks actual matcher entries against the landed converter across fifteen matching policies, three alpha policies, and mixed/transparent-only truncated palettes.
An external integration test compares that converter's coordinate bits against the frozen matcher for the same colors. This separates the oracle from the production semantic module, as the freeze guard requires.
Frozen `spec_diffusion` passes all thirteen tests. The scalar `wasm32-unknown-unknown` cargo check passes.
Full crate formatting, `git diff --check`, frozen-content verification, and semantic syntax verification pass.
The initial full freeze-guard attempt found the forbidden test-only oracle imports; the corrected checks rerun content and syntax without rebuilding its isolated targets.

Commands use `CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target` and `--manifest-path crates/ditherette-wasm/Cargo.toml`:

- `cargo test --test prod_diffusion --test spec_diffusion`
- `cargo test --lib baseline_constructor_adapters_use_landed_converter_without_reordering`
- `cargo check --target wasm32-unknown-unknown`

No benchmark measurements run in this phase.

## Native benchmark baseline

Literal implementation checkpoint: `91cd93207e1935c53e04cd7b9678cdae742e6f99`.
The accepted S26 join excludes unselected converter candidate `b237b746`.
`prod:dither-and-quantize:diffusion:full-image-v1` calls the literal implementation and retains result allocation/destruction inside timing.
The actual paired native worker accepts typed diffusion settings. Request mapping stays outside timing; identity includes all diffusion, matching, palette, alpha, and placement fields.
The registered subject and timed callable match frozen results for 120 kernel/feedback/matching combinations.
`cargo check --manifest-path crates/ditherette-bench/Cargo.toml --bin ditherette-bench` passes using the assigned target cache.

## Deferred scope

Exclusive measurements, selection, and the S28 PR remain unfinished.
The literal baseline and native benchmark registration are recorded before optimization starts.
The coordinator owns integration/tracking and exclusive measurement clearance. S26 source and artifacts stay untouched.

## Bounded candidate checkpoint

The native baseline adapter is `c47419e4`. The candidate retains its literal full-image callable for comparison.
`PreparedDiffusion` reuses the landed `PreparedQuantizer` through crate-private matcher/converter accessors.
Three rows start with source coordinates. Each contribution keeps the frozen tap, axis, f64 calculation, and f32-store order.
Scratch owns exactly `width * 3 * 12` bytes with the current allocator; actual capacity is counted after every fallible reservation.
The private `Processor::dither_and_quantize` dispatches diffusion with source/index preflight and complete-result publication only after successful execution.
The candidate's nine native tests pass, including 10,800 frozen comparisons for the ring and complete processor call, exact-limit/one-under checks, allocation/copy/completion/arithmetic failures, recovery, and disposal.
Height growth adds only owned source and index bytes. Work-row capacity remains unchanged across heights 1, 2, 3, 7, and 19.
Four existing field processor tests, scalar Wasm check, full crate formatting, and diff checks pass.
Public package wiring and candidate benchmark registration follow next. No candidate is selected without the coordinator's measurements.

## Public candidate checkpoint

The delivered S26 dependency `bb36452ca831bad485f924e7ee007f9bbdb1cb0d` joins at `c2e3514b9c3709d46db614aab33d95a309b9d833`.
The join's content delta matches only the accepted S26 verifier repair, tests, and documentation. Thirteen relevant benchmark tests pass.
Private family 2 exposes diffusion through the existing borrowed numeric ABI; the signature stays unchanged.
The package adds canonical kernel, feedback, serpentine, strength, and placement validation and publishes the same durable indexed result.
Forty public/private Node tests pass, including 360 generated frozen diffusion vectors, arithmetic distinctions, metadata, exact budgets, copy failures, and recovery.
Scalar and threads package artifacts build using the assigned S24 target's scalar/threads children. TypeScript and declaration fixtures pass.
Installed-tarball conformance passes in Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, including all 360 diffusion vectors without cross-origin isolation.
WebKit uses the coordinator's existing launcher through `DITHERETTE_TEST_WEBKIT_EXECUTABLE=/home/mia/mia-cx/ditherette/.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit`.
Its dependency aliases remain unchanged; no system packages or copied libraries are added.
The pnpm automatic dependency-install check rejects borrowed node_modules links; build commands use the installed tools directly. Tarball checks disable that automatic installation with `pnpm_config_verify_deps_before_run=false`.

## Candidate benchmark handoff

The native worker accepts `NativeOperation::Diffusion` with complete typed `DiffusionSettings`.
Its accepted subject is `prod:dither-and-quantize:diffusion:full-image-v1`; its candidate is `candidate:dither-and-quantize:diffusion:three-row-v1`.
Both native subjects borrow source and include all preparation, work/output allocation, and result destruction. The candidate wrapper also enforces its stated capacity limit.
The registered native subjects and timed callables pass 120 frozen kernel/feedback/matching comparisons before measurement.
Actual public browser cases use `PublicOperation::Diffusion` and `public:dither-and-quantize:diffusion:package`, calling the package's `ditherAndQuantize`.
The browser protocol validates identity, complete indexed metadata, and rejects unsupported TypeScript claims.
Public complete-call timing remains separate from the borrowed native comparison. There is no equivalent bounded full-image public baseline; initial public evidence can use the same candidate artifact in both roles.
Six browser protocol tests and 28 JS transport/timing tests pass. Their fixtures perform untimed operations or use fake clocks, never actual performance measurements.
No ring optimization is selected and no S28 PR is filed. The coordinator owns fresh paired measurements and the selection checkpoint.

## Declared experiment budget and final validation

The public checkpoint is `e5793547d50d37a2dba5f4403086e15ceee14dae`; native/public candidate adapters are `ffe996bf26062b5ade13d4b554ee9e5cea44a562`.
`crates/ditherette-bench/examples/diffusion_integration_plan.rs` generates typed native or public experiments without measuring them.
Its CLI takes `native|public`, a new output JSON path, and host-load notes. Existing output files are never overwritten.
Use the assigned S24 quantize target explicitly when building or running this example.

The fixed declaration has eight recipes at 65 by 33 pixels with sixteen palette entries, including transparency.
It covers every kernel with both feedback modes. Matching rotates through sRGB Euclidean, Oklab Euclidean, and CIEDE2000.
Scan direction, all three alpha policies, and everywhere/adaptive placement rotate across those recipes instead of multiplying them.
Adaptive placement covers radii one and two. Input contains zero, fractional, and opaque alpha.
Native cases compare the literal full-image subject against the three-row subject with the same borrowed-source scope.
Public cases self-pair the ring package in primed instances, separately from native results. Both public roles must use the same immutable package artifact.

Before measurement, the coordinator records fresh artifact identities, workload targets, host load, and exclusive clearance.
The declared budget is two pairs and twenty single-call samples per role, with 50 ms warmup and a 10,000 ms measurement window per worker.
Eight native cases plus eight public cases in each of three engines declare 128 serial workers and at most 2,560 samples.
Collectors can stop after at least five samples when the measurement window expires; twenty is the requested maximum, not a guaranteed count.
Their nominal warmup/measurement windows total 1,286.4 seconds; startup, verification, and single-call overshoot are additional.
There are no automatic retries or extra candidate revisions in this declaration. The coordinator must bound process time separately.
The existing release gate rejects confirmed per-case median regressions above ten percent. Public self-pairs establish evidence, not an optimization comparison.

The generator's focused test passes. It checks identities, round-trip typed validation, control coverage, call scopes, subjects, and fixed counts.
The final installed-tarball run passes all four test entries, including all three engines.
The coordinator's full trusted S18 freeze guard also passes, including independent native/Wasm spec and production compilation, dependencies, and syntax.
The frozen digest remains `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Temporary task-owned root links now use ignored ordinary directories with borrowed child links; no dependency or cache contents are deleted.
The assigned scalar and threads caches remain children of the S24 quantize target. Other agents' targets are untouched.

Built package Wasm SHA-256 values:

- Scalar: `a65aa6fbcce15ec8268c0fce3bbbd4da3f96656dca8e0c9c527730e9ffa49d4e`.
- Threads: `4d02309582a00b46a092df40d7f643535395333bce739fd51d41e33bd6c85afa`.

These files are validated local artifacts, not fresh clean-checkpoint benchmark provenance. The coordinator prepares immutable measurement artifacts after the final commit.
No measurements, candidate selection, PR filing, or issue changes occur in this checkpoint.

## Permanent oracle integration and fresh preparation

The S28 benchmark branch joins the permanent oracle and S27 blue-noise delivery at `4383ede30f398efb56806f8ab1ae9fcfa91bc6c3`.
It retains both modes, exact indexed classification, and immutable first/distinct output snapshots.
The final S27 dependency is `44cbe43546e739f3d11f7b0bd08d7453afb83da1`, including updated S26 and shared-output rejection.
Diffusion kernels and the source-initialized three-row implementation remain unchanged from `8df7b48396aec85bf9c1a289d63973b615e31018`.
Frozen spec, image, and policy files remain unchanged. Yliluoma stays on its sibling branch until the later combined integration.

The new `diffusion_conformance` example binds all 360 original public fixtures to complete independently computed native identities.
It recomputes native frozen output and checks the original indices/warnings before exporting.
The permanent browser flow computes a frozen-only Wasm reference in a disposable context, then closes that context before package initialization.
Actual primed/fresh package adapters match complete target-local indices, palettes, transparency, warnings, and durable output.

Validation at `0fb19ad59e68c5227aa2ca128edab147b801500f` passes all 423 identified cases per engine in Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Those cases include 360 diffusion, 47 quantize, twelve field, and four blue-noise fixtures.
All diffusion native/Wasm outputs agree. The only retained target difference is the already diagnosed `blue-noise-oklab-adaptive2` case.
The run uses immutable comparison snapshots. The subsequent shared-output guard changes no package output; its 28 focused browser/timing tests pass separately.
Seventeen native kernel/processor tests, thirteen protocol/oracle/generator tests, and 69 public/private/transport Node tests pass.
All benchmark targets, scalar/threaded artifacts, TypeScript/declaration checks, both crate format checks, and diff checks pass.
The coordinator's full trusted S18 guard passes at that unchanged Rust/package head, including native/Wasm/threaded isolation and compiler/dependency/syntax/content checks.

Retain `target/s28-validation-01/` as untimed validation evidence, not paired trial provenance:

- `diffusion.json`, `quantize.json`, `fields.json`, and `blue-noise.json` contain complete native identities and frozen outputs.
- `oracle/manifest.json` binds frozen source, dependency features/checksums, compiler/stdlib identities, source inputs, and emitted oracle bytes.
- `oracle-evidence/{chromium,firefox,webkit}-references.json` preserves all native and Wasm references, runtime versions, oracle manifest, and tarball digest.
- The installed tarball SHA-256 is `920a5924cb236cc38990f82e92770c090e626632b5c36efe0bdee6ede02f545f`; the oracle stays outside it.
- Scalar Wasm is `49dbf3a6717ae2d130e22efe7fae8bf2627288b413794193003b1f9278167182`; threads is `c8b8248e8f9d0bcbba0d10b8a74481922d9809e14557f70e638385b9f0bb82d0`.

The coordinator authorizes fresh independent roles and immutable snapshots after this final parent joins.
Prepare them under new `target/s28-trial-01/` from the clean commit containing this declaration, keeping source fixed throughout preparation.
Use the assigned S24 quantize native target and scalar/threads children. Registry caches and previous evidence stay intact.
Use the existing eight-case native/public generator, two pairs, and at most twenty samples per worker.
The complete native plus three-engine public declaration remains 128 serial workers and at most 2,560 samples.
Native roles compare borrowed-source literal full-image against the ring; public roles self-pair the identical ring package.
Leave every measurement worker unstarted for the coordinator. No candidate selection or PR filing occurs here.
