# S36 quantize and separable-field row bands

Issue #78. Branch `impl/v1-s36-fields` starts at validated S34 delivery
`3863cadce82b4d272a1730f3ebad828b18ac4434` in `.worktrees/v1-s36-fields`.
Read the current coordinator execution contract, S36 slice, reuse inventory
`78-79-reuse-inventory.md`, and decisions 24/37/40 before implementation.

## Reuse and ownership

Keep `PreparedQuantizer`, its ordered palette/matcher, and packed converter.
Its allocation-free pixel loop already implements alpha and stable tie rules.
Add a disjoint-output row adapter by extracting that existing row loop once.
The source remains a full immutable view; metadata stays owned by preparation.

Keep the landed perturb arithmetic, field generators, adaptive placement, and
RGBA8 reconstruction. Its existing row method addresses a full output image.
Add a band-local output method with explicit absolute rows. Both methods call
the same pixel loop. Adaptive neighbors still read the original full source;
field identities remain `y * full_width + x`, even for suppressed pixels.
Keep the RGBA8 boundary before separable quantization. Diffusion remains scalar.

This worktree owns `prod/quantize/prepared.rs`, quantize row adapters,
`prod/dither/perturb.rs`, field row adapters, focused tests, an S36 benchmark
subject fragment, and this plan. Root reconciles shared pipeline/registry files.
S35 owns resize/color policy and shared scheduling work. Do not duplicate it.
No public API, execution selector, cache identity, or callback contract changes.

## Steps

1. [x] Add safe band-local kernel adapters and prove exact bytes across disjoint
   output bands, worker counts, strides, alpha/ties, field identities, and adaptive seams.
2. [x] Define bounded execution capacity using existing worker-budget geometry.
   Count each concurrently live perturb converter and other temporary records;
   share immutable quantizer preparation. Prove exact budget and one-under behavior.
3. [x] Define S36 recipes through existing complete-call benchmark subjects.
   Reuse typed native/public mappings and verify frozen output without timing.
   Root owns the measured dimensions, trial matrix, and scheduling selection.
4. [x] Join the shared pooled executor and integrate candidate row scheduling
   through private fragments. Report progress only after joined work on the caller.
5. [x] Compare fresh complete-call host-worker artifacts and select measured cost classes.
6. [x] Prepare the S36-only publication branch, measurement report, and exact native validation for root.
7. [x] Validate the ordinary package and prepare the reviewable stacked PR.

## Validation and budget

The first checkpoint is native exactness and budget evidence, not a speed claim.
Use only fresh local `target/compiler`. No Wasm builds or timing are authorized yet.
Frozen spec and shared image infrastructure remain unchanged. Compare against
frozen output plus the landed scalar path, including metadata and warnings.

Native pool fixtures vary one, two, and four workers and non-dividing band heights.
Exercise all matching tags, alpha modes, Bayer sizes/random/blue-noise fields,
working spaces, adaptive radius larger than a band, and strided source guards.
Keep test images small. A field callback witness proves one global draw per pixel.
Prepared quantization must allocate no per-worker palette or full-image color plane.

Measured crossover policy is deferred. Root predeclares experiment targets and
budget before exclusive runs; at most two candidate revisions follow the contract.
Main-JS preferred remains scalar, and required remains a capability error there.
Real threaded public calls run in a blocking-capable processing host worker.

## First native checkpoint

Band-local quantization and perturbation now call the same landed pixel loops
as their full-image adapters. The new methods accept only their disjoint output
storage and retain the full immutable source view. No scheduler or selection
threshold is active. Shared preparation stays borrowed across all workers.

Focused native validation passes 19 tests. It includes all 15 matching tags and
three alpha modes, exact prepared-capacity and one-under failure, immutable
metadata, six fields, seven spaces, two strengths, and adaptive radii 2 and 8.
Worker counts 1/2/4 and band heights 1/2/3/11 preserve exact frozen bytes.
Every field pixel draws once, including zero strength, and source/output guards
remain intact. Existing scalar, allocation, and progress tests stay green.
The allocation witness also exercises the new quantize row adapter directly.

S35 owns the shared allocation-free pooled executor and fallible capacity-accounted
row work plans. Step 2 joins that helper rather than introducing a competing plan.
This checkpoint proves the domain adapters and existing preparation budgets;
combined per-worker execution preflight remains pending. Native scoped threads
here are exactness fixtures only, not benchmark subjects or timing evidence.

## Complete-call recipe checkpoint

`crates/ditherette-bench/examples/support/row_fields.rs` supplies six recipes.
They cover quantize, perturb, and separable calls with cheap and perceptual matching,
random and blue-noise fields, and adaptive full-source reads. Palette sizes are
16 and 64. The fragment chooses no dimensions, workers, thresholds, or trial budget.
It reuses existing production subject IDs and native/public request types.
The existing field-call adapters retain input copies, preparation, durable copies,
and result destruction. The existing native quantize subject includes preparation
and result allocation; browser evidence will use the actual public Processor call.

The new untimed `row_field_adapters` test passes all six recipe comparisons.
It checks native/public identity equality, exact frozen output and metadata,
unique settings identities, and unchanged input. No shared registry changed.

## Shared execution checkpoint

Joined S35 helper `d0af42d46d3d2726345ea8b7be281724fb07d00d` without conflicts.
The quantize and field adapters now accept caller-preflighted `RowBandBuffers<()>`.
They share preparation/source, write disjoint output, and report only joined rows.
Field workers retain the landed converter construction and charge one converter
record per active worker. No per-worker heap scratch or image color plane exists.
The full native exactness matrices now exercise this actual executor instead of
test-owned scoped threads. Both tests pass in scalar and `threads` builds.
They also prove callbacks stay on the caller and reach the full output height.
Combined capacity preflight and allocation-failure witnesses are the next checkpoint.

## Worker-capacity checkpoint

Joined S35's allocation-free band iterator at `20fc297b9bd376883206007cd6a26bb5f139a626`.
Field preflight now reserves shared row metadata plus one existing converter per
active worker. It uses the existing iterator and `WorkerBudget::active_workers`.
`try_band_buffers` checks that combined requirement before the first reservation.
Temporary converter capacity remains charged until every worker joins; no unused
scratch allocation stands in for that stack storage. Quantization reuses the same
row metadata with its one shared `PreparedQuantizer` and no worker converter copy.

The allocation fixture checks 1/2/4 active workers, exact and one-under budgets,
every metadata reservation failure, callback failure after the first joined batch,
untouched later bands, and successful reuse for perturb then quantize. It checks
the independent caller-owned source/RGBA8/indexed/preparation charge separately.
Public pipeline preflight and transactional publication still await root integration.
This fixture does not claim those unwired public behaviors are implemented.

Validation passes 33 focused scalar tests and three focused threaded tests.
All jobs have exited. Only this worktree's native `target/compiler` was used.
No timing, Wasm compilation, threshold selection, or public selector was added.

## Public candidate checkpoint

Joined final S34 report head `d4531667e1158c2068f30614f40c9d39f8c5313e`
and S35 private policy seam `f437a18578f786be2ed7cd4cba48f7537db54e32`.
Private indexed policy now schedules complete `perturb`, `quantize`, separable
`ditherAndQuantize`, and `process` calls. Public defaults remain scalar.
One row plan serves perturbation and quantization sequentially. Preflight counts
its assignment ownership and concurrent converters with all other live call data.
The existing scalar converter charge counts the first field worker only once.
The call drops execution metadata before cache retention and durable output.

All 34 native threaded library tests and 17 focused scalar integration tests pass.
New complete-call tests vary 1/2/4 workers and 1/2/3-row bands. They check unchanged
metadata, policy-independent cache hits, caller-thread callbacks, callback failure
after joined work, final-copy failure, no failed publication, and successful reuse.
Exact full-call minimum budgets succeed; one-under fails before processing stages.
Existing adapter matrices still match frozen bytes and full-source adaptive seams.

S35 identified one shared pressure-order follow-up: capacity charging must let
`prepare` discard idle scratch before evicting LRU entries. Its owner is applying
that correction. The final integration joins it before benchmark delivery.
Root owns host-worker builds and complete-call measurement. No timing or Wasm
build has run in this worktree.

## Measured selection checkpoint

Delivery continues on `delivery/v1-s36-fields` in `.worktrees/v1-s36-delivery`.
The shared stage-override seam is joined with ancestry at `bcffc184`.
Root ran Chromium trial 02 and Firefox trial 03 in `.worktrees/v1-s35-37-bench`.
Their retained `target/rows-trial-03/combined-analysis.{json,md}` records exact
report hashes, identities, per-pair counts, medians, and all inconclusive cases.
Each engine completed 200 serial workers and 100 exact scalar/row pairs.
Separate reference probes are not additional scalar/row pairs.

The accepted role forces scalar inside the same threaded developer artifact.
This evidence does not compare the ordinary scalar package against threading.
Cold complete calls include boundary copies, preparation, and durable results.
Warm final hits remain overhead controls, not kernel acceleration evidence.

Automatic S36 scheduling uses conservative cost-class heuristics:

- At least 769 by 513 pixels: sRGB random/everywhere fields and preserve-alpha
  sRGB Euclidean quantization with 15 visible entries plus trailing Transparent.
  Four available workers select height 128. Pools of two or three select two
  workers at height 32. Both configurations have common-engine passing gains.
- At least 65 by 49 pixels: Oklab blue-noise/adaptive-radius-two perturbation.
  Separable matching additionally requires Oklab Euclidean preserve-alpha and
  63 visible entries plus trailing Transparent. Select two workers, height 32.
- Keep small inputs, zero strength, other recipes, direct Oklab quantization,
  Process field scheduling, and one-worker pools scalar. Diffusion stays scalar.

Seed and palette RGB values remain independent. Nonzero strength and adaptive
threshold/softness retain the same structural loop class. These heuristics use
representative measurements; they do not predict every image or control value.
S36 sRGB field samples were 5/7 per pair in Chromium and 5/5 in Firefox.
Oklab field samples were 20/20 in Chromium and 5-6/8 in Firefox.
The existing minimum-five collection rule passed; requested 20 is not a new gate.
Genuine noisy gates remain incomplete and do not select automatic execution.

Private forced scalar/row overrides remain independent per stage. Normal scalar
builds select scalar. Existing preflight charges actual worker ownership before
dispatch; callback handling and success-only cache publication stay unchanged.

Focused tests cover measured boundaries, pool capacities 1/2/3/4/8, cost-class
exclusions, actual automatic public calls, scalar bytes and metadata, caller
progress, callback failure/retry, and policy-independent final-cache hits.
Native validation uses only this delivery worktree's `target/compiler`.
Both release library suites pass 42 tests, with and without `threads`.
Nine focused threaded integration tests pass across `prod_quantize_row_bands`,
`prod_field_row_bands`, `prod_field_band_allocation`, and `prod_processor_fields`.
No Wasm build, measurement, push, or PR operation is authorized in this task.

## S36-only publication checkpoint

The publication branch is `delivery/v1-s36-fields-final` in
`.worktrees/v1-s36-publication`. Its base `16136471` contains S36 selector
`1513153d` and final S35 `a8418904`. It adds the isolated developer-getter
cleanup as `ddcd7477`. The joined branch remains unchanged at `5933ae55`.
This publication diff adds no S37 implementation, helper, tests, or callsite.

The S36-only measurement report is
`docs/plans/ditherette-v1/s36-row-bands.md`. It retains every cold and warm cell,
actual sample counts, all noisy controls, source identities, and report hashes.
Time-capped passing cells remain valid under the predeclared minimum-five rule.
Ordinary-package browser validation completes against combined source `dc81818a`; see the final disposition below.

Fresh publication-worktree validation passes 44 scalar and 45 threaded release
library tests. Both configurations also pass nine focused integration tests:
`prod_quantize_row_bands`, `prod_field_row_bands`, `prod_field_band_allocation`,
and `prod_processor_fields`. Only this worktree's new `target/compiler` is used.
All owned native jobs have exited.

The returned `.worktrees/v1-s36-delivery/target/compiler` was a real directory
with no symlinks, active jobs, or custom evidence. Removing its rebuildable
contents reclaimed 171 MiB. Source and all retained trial evidence stay intact.

## Final delivery disposition

Publication joins S35 PR 126 head `6bbe113b99a08f2a11ade6296ddf7f25e32e041b`, then rebases with merge ancestry preserved
onto published `delivery/v1-s35-resize`. Only the S36 runtime family remains in its PR diff.
Production field/quantize and shared build inputs match tested `dc81818a`; the remaining differences belong to S37 mixing.
No runtime or build input changes after the exact native subset checks above.

The coordinator's combined ordinary package passes the trusted frozen guard. Tarball SHA-256 is
`1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
Its automatic host fixture passes 23 checks, including nine cases per Chromium/Firefox engine and actual Oklab staged composition.
`.plans/77-79-auto-host.md` records the exact invocation and completed browser cleanup.

All slice TODOs are complete. S41 retains inconclusive small/warm controls, unmeasured recipe classes, automatic-policy overhead follow-up,
and the pinned WebKit threaded cleanup gate. Untimed package equality does not turn those performance gates into passes.
The shared paired evidence contains 400 serial workers, 7,215 samples, and 200 exact production comparisons across both engines.
S36's complete subset, actual time-capped counts, and report hashes remain in its measurement report.
