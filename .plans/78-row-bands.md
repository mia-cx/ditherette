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
4. [ ] Join the shared pooled executor and integrate candidate row scheduling
   through private fragments. Report progress only after joined work on the caller.
5. [ ] After root quiet clearance, compare fresh complete-call host-worker artifacts.
   Select only measured wins, record scalar winners, and hand off a reviewable PR.

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
