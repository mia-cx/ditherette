# Resize performance playbook

This document records the resize optimization patterns that have worked in
Ditherette so future filters can start from proven moves instead of rediscovering
local perf-loop history.

## Benchmarking rules that made the work reliable

- **Measure the product path.** Prod benchmark subjects call the public one-shot
  resize APIs (`resize_*_rgba8_into`) because the app does not repeatedly resize
  with the same dimensions.
- **Use manifest profiles as source of truth.** Run `ditherette-bench run
  nearest`, `run area`, or `run bilinear`; do not hand-roll scale/fixture flags
  for acceptance.
- **Correctness comes first.** Nearest and exact integer area stay byte-exact.
  Fractional area and production bilinear use bounded RGBA color-distance checks
  against the spec oracle.
- **Treat aspect-preserving cases as representative.** Anisotropic cases remain
  useful guardrails, but app decisions should not be justified only by
  aspect-ratio-changing wins.
- **Promote baselines only after acceptance.** Candidate comparisons use plain
  `run <profile>`; `--replace-baseline accepted` happens only after an accepted
  change.

## Cross-filter lessons

- **Keep prod packed RGBA8-only.** Normalize before prod resize. Avoid growing
  strided prod kernels or generic pixel-format branches.
- **Keep spec and prod independent.** Prod can duplicate formulas, but must not
  import `spec` internals.
- **Optimize at the right boundary.** Caller-level identity pass-through is
  better than allocating an output and then copying. If the caller cannot skip
  allocation, keep filter-local identity copies simple and local.
- **Prefer local branch shapes over generic helpers.** Shared same-size copy and
  shared exact-scale classification both regressed represented cases. Reuse is
  only worth it when it preserves each filter's hot branch layout.
- **Do layout/path work before micro-tuning.** The winning changes were plan
  metadata, path classification, and separable kernels; tiny arithmetic rewrites
  often regressed or vanished in noise.
- **Document rejects near the code.** Closed ideas are valuable because many
  plausible resize optimizations are workload-sensitive.

## Nearest-neighbor: what worked

Nearest is dominated by copying selected source pixels, not numeric filtering.
The successful shape is a one-shot plan plus specialized copy paths:

- **Precompute axis maps in a plan.** Store x byte offsets and y coordinates once
  per one-shot call so inner loops avoid coordinate math.
- **Copy pixels as unaligned `u32` words.** Inputs/outputs remain RGBA8 bytes;
  the word copy is only an internal nearest kernel trick.
- **Classify scale shape early.** Route exact downscale, exact upscale,
  near-identity downscale, other downscale, and upscale separately.
- **Use row-repeat for nearest upscales.** Write the first output row for a
  repeated source y, then copy that output row for duplicate y coordinates.
- **Use span copies only for near-identity downscales.** It helps when x mapping
  has long contiguous runs; broader span-copy gates regressed larger downscales.

Do **not** start future nearest work with a separable two-pass image. X and y are
already planned independently; materializing an intermediate buffer adds a full
write/read without reducing sampling work. Only revisit if a future benchmark
profile exposes a shape where axis-only materialization clearly avoids more work
than it adds.

## Area: what worked

Area has two very different performance regimes: exact integer scales and
fractional coverage.

### Exact integer area

- **Bypass fractional planning.** Identity, exact integer downscale, and exact
  integer upscale leave the fractional planned-coverage path immediately.
- **Use integer block sums for common exact downscales.** Power-of-two cases use
  specialized integer accumulators, preserving exact output while avoiding the
  generic weighted grid.
- **Use packed pixel-repeat for exact upscales.** Shared packed RGBA8 repeat
  rows are useful here because exact upscale semantics collapse to pixel blocks.

Exact integer area is not a good separable target: output blocks are disjoint and
already read each source pixel once. A two-pass scratch image would add memory
traffic without reducing coverage work.

### Fractional area

- **Separable y-then-x coverage won.** Filtering y coverage into one f32 scratch
  row, then applying x coverage, avoids repeated `x_overlap * y_overlap` grid
  work per output pixel. This produced large wins on represented fractional
  aspect-preserving cases while staying within the bounded area oracle.
- **Keep f32 accumulation for bounded fractional output.** Exact integer paths
  remain byte-exact; fractional area accepts bounded visual drift for speed.
- **Keep y span layout local for now.** Flattening y spans previously regressed;
  the accepted separable path still uses the existing `AxisOverlap` layout.

Next area ideas should build on the separable path: gate it by shape/output size,
try scratch reuse, specialize common one-/two-overlap kernels, then consider
normalizing weights only inside the accepted separable shape.

## Bilinear: what worked

Bilinear became fast once production stopped chasing byte-for-byte f64 grouping
and moved to one bounded, separable implementation.

- **Cache compact nonzero support taps in the plan.** Precomputing x/y tap lists
  removed hot coordinate/filter work. Preserving duplicate clamped edge taps kept
  bounded correctness stable.
- **Use one production implementation.** Avoid a separate `bilinear-fast`; the
  single prod path is the optimized path.
- **Use vertical-first f32 scratch.** The separable y-then-x kernel closed most
  of the old scalar gap under bounded correctness.
- **Bypass same-size output before planning.** Identity resizes copy packed RGBA8
  bytes without building the plan.
- **Specialize identity-axis cases.** Width-only and height-only resizes skip the
  unnecessary half of the separable scratch/gather work. These cases are mostly
  diagnostic for the app, but they guard against future path regressions.

Rejected bilinear directions worth remembering:

- normalized `first + f32 weights` tap layout regressed;
- flattened tap arrays plus per-output ranges regressed;
- edge tap coalescing changed contribution semantics;
- fixed 2x2/two-tap kernels regressed represented upscales;
- broader RGBA unrolling regressed or did not help.

## Checklist for future filters

1. **Define the oracle and correctness contract.** Decide exact vs bounded before
   optimizing.
2. **Add a manifest profile.** Include representative app scales, identity, and
   any diagnostic edge cases separately enough that they are visible in results.
3. **Benchmark the one-shot public path.** Add a planned/hot subject only if the
   app actually reuses plans.
4. **Normalize the prod boundary.** Assume packed RGBA8 internally; convert before
   prod resize rather than branching inside kernels.
5. **Add the cheapest bypasses first.** Caller-level identity, filter-local
   identity, exact integer scale, and whole-row copies should be tested before
   complex kernels.
6. **Precompute per-axis metadata.** Coordinate maps, support taps, spans, and
   scale classes usually beat recomputing inside pixel loops.
7. **Split major shape classes.** Downscale/upscale/identity/axis-identity and
   exact/fractional paths often want different kernels.
8. **Try separability only when it removes repeated work.** It helped bilinear
   and fractional area; it is unlikely for nearest or exact area.
9. **Prefer row/scratch locality over full intermediate images.** One-row scratch
   buffers were enough for bilinear and area.
10. **Only then micro-tune.** Reciprocals, unrolling, flattened layouts, and
    helper extraction should be judged after the winning path shape is settled.
11. **Keep rejects close to the code.** If a plausible idea loses, write a
    concise `REJECT(perf)` with the profile and reason.
