# Ditherette v1 implementation PRD

Status: approved by Mia on 2026-09-07. Implementation proceeds through unmerged stacked PRs.

[Read the 45-slice dependency plan](slices.md).

## Problem statement

Ditherette has a substantial Rust/Wasm port, but its public processing path currently only resizes images. Quantization, dithering, memory ownership, packaging, and rollout do not yet form the complete reusable package we agreed to ship.

The implementation must cover all current website modes and all coherent extra modes already in the Rust specs. It must remain usable as scalar Wasm without threads or shared memory.

## Solution

Ship the unscoped browser ESM package `ditherette`, backed by the internal `ditherette-wasm` crate. It accepts cropped RGBA8 and a caller-supplied palette. It exposes asynchronous `createDitherette()` initialization and five synchronous processing methods:

- `process` returns indexed output after resize, dithering, and quantization.
- `resize` returns RGBA8.
- `perturb` returns palette-independent RGBA8 for separable fields.
- `quantize` returns indexed output.
- `ditherAndQuantize` returns indexed output for every supported dither family.

Complete readable reference implementations before freezing them. Copy the implementations from `spec/` into mirrored `prod/` modules, then improve only production using measured evidence. Deliver the work as unmerged stacked and parallel PRs.

Mia authorized end-to-end implementation after reviewing this PRD and its slices. Operational holds below remain in force.

## User stories

1. As a browser developer, I can initialize a processor with scalar defaults and explicit initialization options.
2. As a caller, I can process cropped RGBA8 into indexed output in one complete call.
3. As a caller, I can resize alone with every current and specified extra resize recipe.
4. As a caller, I can quantize alone using the supplied palette and valid color/metric choices.
5. As a caller, I can perturb alone or use fused dithering and quantization for the selected algorithm.
6. As a caller, I receive stable palette indices, normalized palette metadata, transparency information, and warnings.
7. As a caller, I get the agreed alpha behavior without mutation of my input or invalidation of earlier outputs.
8. As a host, I can bound processing allocations while repeated calls reuse private prepared data, stages, and scratch.
9. As a host, I get structured errors, predictable callback behavior, isolated instances, and idempotent disposal.
10. As a website user, I get progress, cancellation, and protection from stale results through the host's worker.
11. As a host, I can request optional threads without changing deterministic results or requiring them for scalar processing.
12. As a developer, I can install a complete browser package whose assets work independently of this repository or another project.
13. As a maintainer, I can release the `0.x` beta line through reproducible package-owned publishing automation.
14. As a maintainer, I have immutable readable references for every semantic module, kernel, executable adapter, and export.
15. As a maintainer, I can prove an optimization improves current measured performance without silently changing accepted output.
16. As a website maintainer, I can adopt Wasm behind a developer flag, use faithful temporary fallback, and retire TypeScript deliberately.
17. As Mia, I can review granular stacked PRs while the agent advances dependencies without merging or activating releases.

## Decision authority

Read the relevant resolved decision before implementing a slice. These issue resolutions are the authority; old spec prose and historical optimization proposals may be stale.

| Topic | Authoritative decision |
|---|---|
| Processing ownership | [Choose the Rust/Wasm boundary and correctness model](https://github.com/mia-cx/ditherette/issues/23) |
| Completion scope | [Define what production-complete means for ditherette-wasm](https://github.com/mia-cx/ditherette/issues/34) |
| Public contract and addenda | [Choose the stable process API and settings contract](https://github.com/mia-cx/ditherette/issues/35) |
| Package/build/publishing | [Choose npm package layout and toolchain ownership](https://github.com/mia-cx/ditherette/issues/36) |
| Color representation | [Choose intermediate color precision and memory layout](https://github.com/mia-cx/ditherette/issues/25) |
| Semantic architecture | [Choose production quantization and dithering architecture](https://github.com/mia-cx/ditherette/issues/40) |
| Cache and memory | [Choose cache ownership and memory-budget policy](https://github.com/mia-cx/ditherette/issues/37) |
| Progress/cancellation | [Choose cancellation and responsiveness guarantees](https://github.com/mia-cx/ditherette/issues/24) |
| Release validation | [Set Rust/Wasm production release gates](https://github.com/mia-cx/ditherette/issues/38) |
| Website adoption | [Choose rollout, fallback, and TypeScript retirement policy](https://github.com/mia-cx/ditherette/issues/39) |

The [completed Wayfinder map](https://github.com/mia-cx/ditherette/issues/33) remains closed. After sign-off, file a separate implementation PRD and attach these slices as its native sub-issues.

## Implementation decisions

### Semantic scope and freeze

Keep `image/` as shared storage infrastructure. Mirror semantic modules under `spec/` and `prod/`, including color, palette, quantize, dither, pipeline, and existing resize/tiling modules.

Each semantic kernel and export needs a readable naive reference. Executable adapters and control modules need reference compositions or readable state models where mathematics alone does not describe them. References do the work directly, without optimization shortcuts.

Complete the whole reference contract, record a named commit and content digest, then enforce the freeze. Already-landed production kernels and shared helpers remain production. Reuse them when integrating the package and implementing missing kernels; do not replace them with naive copies or repeat their optimization work. Every later semantic change requires a versioned recipe or named mode; optimization cannot edit the oracle. Enforce semantic independence in both directions: spec cannot import production, and production cannot call spec as its implementation.

Required equalities include result metadata and warnings:

```text
process(input) = ditherAndQuantize(resize(input))
ditherAndQuantize(input, separable mode) = quantize(perturb(input))
```

The RGBA8 boundary in separable composition includes its rounding and clipping. An optimization cannot bypass that boundary and claim exact equivalence.

### Included algorithms

| Family | Required coverage |
|---|---|
| Resize | Nearest, area, bilinear, Catmull-Rom bicubic, Lanczos2/3, trilinear; specified support policies and anchors |
| Working spaces | sRGB, linear sRGB, Oklab, OKLCH, D65 CIELAB, CIELCH, full-range BT.601 YCbCr |
| Matching | Euclidean, circular hue, CIEDE2000, CompuPhase, Rec.601, Rec.709 in their valid tagged combinations |
| Separable fields | Bayer 2/4/8/16, deterministic random, blue noise |
| Diffusion | Floyd-Steinberg, Sierra, Sierra Lite, Atkinson |
| Mixing | Existing two-color ordered Yliluoma recipe with adaptive placement |
| Placement | Everywhere and palette-independent adaptive placement with the agreed eight-neighbor calculation |

All public processing modes have a scalar implementation. Seeds, worker counts, and tile sizes never change exact results. Diffusion stays scalar and uses three-row production error scratch.

### Pre-freeze findings included in this draft

These are concrete defects or incomplete semantics found during slicing. Plan sign-off covers resolving them before freeze.

1. Production color currently writes f32x4. Replace it with the already approved packed f32x3 representation and byte alpha.
2. The current blue-noise threshold table is exactly transposed Bayer 8. Correct it before freezing. Use a primary-source deterministic readable offline generation recipe. Specify numerical rank/spectral acceptance criteria before generating the asset, then retain fixed parameters/seed, its digest, and the numerical analysis. Freeze both generator and asset. This does not add a public void-and-cluster algorithm.
3. Forward color conversions exist; inverse conversions and explicit reconstruction conventions are incomplete. Specify clipping, byte rounding, neutral hue, and domain limits in the reference slices.
4. Adaptive placement currently normalizes by palette ranges. Define fixed space ranges from the documented coordinate domains, record their derivation, and verify palette independence.
5. Preserve the existing Yliluoma componentwise coordinate interpolation, including its hue-coordinate behavior, and document it. Do not silently replace it with shortest-arc interpolation. Its selected distance metric still applies.
6. Define the exact global-index random sequence and draw count before freeze, then use that same sequence in every production execution policy.

### Runtime and package

The linked decisions own exact defaults and contracts. In particular:

- Request objects and structured errors stay typed. Invalid color/metric combinations fail before output allocation.
- Inputs remain caller-owned; public results are durable JS-owned typed arrays.
- Private allocations include plans, palettes, working images, stages, and scratch. The approved limits count capacity and cover transient work.
- Cache keys derive from 256-bit content digests and normalized settings. New entries publish only after a successful call.
- `dispose()` is idempotent. Calling processing after disposal fails. Wasm page high-water behavior is documented.
- Optional progress callbacks report stages/work counts, throttle within-stage events to 50 ms, and reject reentry/disposal.
- Each initialized processor owns isolated state. The loading implementation must reconcile this with actual wasm-bindgen module/memory ownership.
- Threads are optional; scalar loads only its required artifacts. The wrapper supports custom Wasm initialization.
- The internal crate owns builds/tests; the public package owns wrapper/distribution/publishing. Generated artifacts stay ignored.

### Website boundary

The website keeps decode, crop, palette selection, UI, source/settings identity, persistence, rendering, export, feature gating, and fallback orchestration.

The Wasm path stays behind a developer flag until Mia accepts it as stable enough. Users do not choose or compare backends. Fallback during rollout only covers initialization/capability failures and faithfully supported TypeScript requests.

Use existing status, progress, and error presentation. These slices do not redesign the UI. If implementation reveals a non-trivial new UI requirement, isolate it rather than inventing a design during AFK work.

## Execution contract

Every implementation agent must read this section and its slice before acting.

### Stacks and worktrees

1. Work in an isolated clean worktree. Preserve the root worktree and all user-owned edits.
2. Start from the validated prerequisite commits. Independent slices may branch from one shared checkpoint.
3. Base a child PR on its immediate parent branch. For multiple prerequisites, create a documented integration join containing every required SHA before starting dependent work.
4. Record the issue, branch, PR URL, base SHA, head SHA, dependency SHAs, validation, and benchmark artifacts in a stack ledger.
5. Push real reviewable PRs. Leave every PR unmerged and auto-merge disabled. Do not publish packages, create release tags, deploy, or activate rollout.

S01 establishes the inherited port as the stack anchor. At drafting time, the remote port is 448 commits ahead of main with one main-only commit. Recheck before acting. Preserve existing work rather than reconstructing those commits as new slices.

When a parent changes, restack descendants, verify their actual dependency ancestry, and rerun checks invalidated by the change. Preserve the frozen spec content checkpoint through rebases.

### Dependency availability without merges

Implementation availability means validated prerequisite code is present in the child's ancestry. It does not require a merge.

Keep implementation issues open while their PRs are unmerged. When a dependency becomes available, record its PR and exact SHA on the dependent issue, materialize its base/join, and remove that satisfied native blocking edge. The original dependency list remains in the issue body and ledger.

Use an `implementation:available` label for slices whose PR has satisfied acceptance criteria. The coordinator claims work through the ledger so uniformly assigned issues do not invite duplicate agents.

An upstream change invalidates affected readiness. Restore blocking state until the child is restacked and revalidated. PR references may include the slice's usual closing directive; no issue closes merely because its code is available.

### Parallel ownership

The coordinator assigns disjoint module/worktree ownership before dispatch. Agents do not share a mutable working tree or build output directory.

Centralize public export registries, workspace manifests, and integration joins when sibling changes overlap. A slice owns its mode implementation and registration fragment; the coordinator reconciles common registries at the join.

Use GPT-6-astra with high reasoning for every implementation subagent. At most three run beside the coordinator with the current four-agent capacity. Dependencies and actual file overlap may lower concurrency.

### Exclusive measurements

There are two phases:

```text
Implement/test in parallel
  -> stop dispatch and drain agents/builds/tests
  -> acquire the shared benchmark lock
  -> run one ditherette-bench process and its owned transport
  -> wait for all benchmark children to exit
  -> release lock and resume implementation
```

Never run two ditherette-bench processes. Never run implementation agents, compilers, builds, or test suites during a benchmark measurement. Prepare accepted/candidate artifacts first. A process lock prevents competing benchmarks; the coordinator must separately enforce agent quiescence.

The lock must span all worktrees and remain held for the complete run, including browser children. Fresh cross-revision trials use an external coordinator that launches accepted/candidate benchmark executables sequentially under the shared lease. Never launch a second ditherette-bench from a running ditherette-bench. Use an OS-managed lock so process exit releases ownership. Do not delete another process's lock or terminate unrelated work to acquire it.

Record unrelated host load if encountered; thermal variance still requires fresh paired measurements. Existing machine-wide activity is not permission to stop another project's work.

### Optimization inside slices

First identify what is already implemented. Keep landed optimized kernels, plans, and shared helpers in their production paths and reuse their existing call graph. Implement only missing modes and required integration. A genuinely missing semantic implementation starts with its frozen `spec/` copy, with only mechanical import/module changes, before optimization. Record that baseline separately and verify exact outputs. Reuse existing production helpers where their semantics fit; do not rebuild their optimizations. Extend ditherette-bench subjects before attempting new optimizations.

Restoring unchanged landed implementations is not a new optimization candidate. Preserve their established exact or bounded behavior and record actual reference differences. Future changes still require the agreed exactness or visual-approval process. Historical copied-baseline commits remain evidence, not instructions to replace landed production again.

Measure the accepted implementation and candidate on the same machine/browser in paired alternating runs. Include full-call copies, hashing, and preparation where relevant. Keep one-call latency separate from throughput. Report cold/warm application caches separately.

Set targets per filter and workload from freshly measured baselines, accounting for taps, scale ratio, setup cost, and image size. Nearest-neighbor and trilinear do not share a latency target. Record the target and measurement budget before experiments.

Use one baseline/candidate comparison for a promising optimization, with at most two candidate revisions per algorithm slice by default. Repeat only noisy or inconclusive measurements within that budget. Stop when the target is met or further gains lack evidence. Keep the exact baseline when an experiment loses; do not force an optimization for every scalar kernel. Final conformance and per-case regression gates still apply.

Exact optimizations can proceed AFK. If a candidate differs, preserve its code/images/metrics separately, retain the exact implementation, and continue independent work. Do not mark the non-exact candidate accepted without Mia's visual approval.

For the first release, adapters compare equivalent operations against fresh TypeScript measurements; spec-only extras establish their own baseline. Later gates compare accepted production revisions. The release gate blocks confirmed per-case median regressions above 10%. Missing required comparisons are incomplete evidence. Historical timings do not replace a fresh measurement of accepted code.

### Operational holds

All 45 slices are AFK for their stated code-preparation deliverable. Human acceptance remains required where the decisions already require it:

- A non-exact production candidate waits for visual approval; the exact path continues.
- Package-size growth above the accepted budget's threshold waits for review.
- Enabling Wasm as the default waits for Mia's stability acceptance.
- Retirement activation waits for acceptance of the actual initial rollout and no blocking regressions.
- Publishing, deployment, merges, and release tags are outside this execution authorization.

S44 and S45 prepare separate held PRs. Their preparation can finish without satisfying their activation gates. A ready diff is not an accepted rollout.

## Testing decisions

Tests exercise reference mathematics and caller-visible behavior. Reuse the existing Rust reference/prod tests, benchmark verifier, TypeScript worker tests, and browser fixtures.

- Reference slices use known vectors and tiny independently calculated examples, especially ties, alpha, hue seams, edges, rounding, and random identity.
- Production slices compare every new public mode against the frozen oracle before and after optimization.
- Runtime tests verify memory accounting, eviction, input/output ownership, isolated instances, callbacks, disposal, and failures.
- Browser tests exercise Chromium, Firefox, and WebKit; threaded capability paths are tested where supported. Use package-owned fixtures only.
- Website tests verify the existing processing flow and the approved cancellation/fallback behavior without redesigning controls.
- Package checks install the packed tarball and resolve the actual published asset layout. Publishing workflows are validated without publishing.

A frozen checkpoint plus output tests protects semantics. Tests that only mirror an implementation's internal control flow do not count as independent correctness evidence.

## Completion criteria

The implementation preparation is complete when all slice acceptance criteria have evidence, every required commit appears in the final integration ancestry, and all PRs remain reviewable and unmerged.

The final report identifies the package artifact, modes, browser/tool versions, conformance checks, fresh performance comparisons, memory checks, sizes, rejected optimization candidates, and any pending human gates.

Code readiness, package publication, website rollout, and TypeScript retirement are distinct states. The report must not conflate them.

## Proposed issue metadata

After sign-off, create one new parent PRD and 45 native sub-issues.

| Field | Parent | Slices |
|---|---|---|
| Issue type | Existing `📝 prd` | Existing `📋 task` |
| Milestone | Existing `v1` | Existing `v1` |
| Assignee | `mia-riezebos` | `mia-riezebos` |
| Execution label | None required | Proposed `implementation:afk` |
| Readiness label | None required | Proposed `implementation:available`, only after verified delivery |
| Project | Unassigned | Unassigned |

The current GitHub token cannot read Projects. Project assignment is unnecessary for the dependency graph, so no permission change is requested.

## Out of scope

- Merging any PR, publishing npm, tagging a release, deployment, or activating rollout.
- Caelestis or another external project as a release or smoke-test requirement.
- New public algorithms beyond the settled scope, palette generation, color grading, or UI redesign.
- Publishing the Rust crate, CJS/Node support, or exposing raw Wasm glue as public entrypoints.
- Promoting an unapproved non-exact optimization.
- Editing the frozen spec to make a production candidate pass.
- Modifying the root worktree's existing user-owned changes.

## Review checkpoint

Mia approved this PRD, the 45 slices, their dependencies, and pre-freeze corrections on 2026-09-07. She later explicitly directed restoration of landed optimized kernels and shared helpers. The plan covers remaining work, not rebuilding landed implementations. Literal spec-to-prod copies apply to missing implementations; existing production and shared code should be reused. Bounded filter-specific optimization, GPT-6-astra agents at high reasoning, and all operational holds remain in force.
