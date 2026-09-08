# Ditherette implementation slices

Status: approved on 2026-09-07. [Read the parent PRD and execution contract](README.md).

All slices are AFK for code preparation. S44 and S45 produce held PRs; their operational gates remain unsatisfied until Mia accepts the rollout.

Dependencies below are code prerequisites. They unblock when validated commits are in the dependent branch, without merging. Original dependencies stay documented even after native GitHub blocking edges are cleared.

The reference slices are intentionally a separate prerequisite phase. This follows the required spec freeze; production slices then carry each behavior through its package method, conformance, and benchmarks.

## Index

Ready means the slice has validated implementation in an open, unmerged PR. It does not mean merged or released.
The coordinator updates Progress and PR when work starts, a PR opens, or validation changes. Dependencies and acceptance criteria stay unchanged.

| Slice | Deliverable | Prerequisites | Progress | PR |
|---|---|---|---|---|
| [S01](#s01) | Establish the inherited port stack and validation baseline | None | Ready | [#75](https://github.com/mia-cx/ditherette/pull/75) |
| [S02](#s02) | Move builds and package ownership into their settled workspaces | [S01](#s01) | Ready | [#88](https://github.com/mia-cx/ditherette/pull/88) |
| [S03](#s03) | Define reference requests, results, errors, and mode inventory | [S01](#s01) | Ready | [#89](https://github.com/mia-cx/ditherette/pull/89) |
| [S04](#s04) | Enforce exclusive benchmark execution across worktrees | [S01](#s01) | Ready | [#90](https://github.com/mia-cx/ditherette/pull/90) |
| [S05](#s05) | Add typed three-way benchmark verification | [S03](#s03), [S04](#s04) | Ready | [#96](https://github.com/mia-cx/ditherette/pull/96) |
| [S06](#s06) | Measure fresh accepted and candidate performance pairs | [S05](#s05) | Ready | [#102](https://github.com/mia-cx/ditherette/pull/102) |
| [S07](#s07) | Complete sRGB, linear RGB, and YCbCr reference round trips | [S03](#s03) | Ready | [#91](https://github.com/mia-cx/ditherette/pull/91) |
| [S08](#s08) | Complete Lab and LCH reference round trips | [S03](#s03) | Ready | [#92](https://github.com/mia-cx/ditherette/pull/92) |
| [S09](#s09) | Specify supplied palettes, alpha handling, and warnings | [S03](#s03) | Ready | [#94](https://github.com/mia-cx/ditherette/pull/94) |
| [S10](#s10) | Specify direct quantization for every valid metric | [S07](#s07), [S08](#s08), [S09](#s09) | Ready | [#97](https://github.com/mia-cx/ditherette/pull/97) |
| [S11](#s11) | Audit and complete the reference resize family | [S03](#s03) | Ready | [#93](https://github.com/mia-cx/ditherette/pull/93) |
| [S12](#s12) | Specify palette-independent adaptive placement | [S07](#s07), [S08](#s08) | Ready | [#95](https://github.com/mia-cx/ditherette/pull/95) |
| [S13](#s13) | Specify palette-free Bayer and random perturbation | [S12](#s12), [S09](#s09) | Ready | [#98](https://github.com/mia-cx/ditherette/pull/98) |
| [S14](#s14) | Repair the blue-noise reference before freezing | [S12](#s12), [S09](#s09) | Ready | [#99](https://github.com/mia-cx/ditherette/pull/99) |
| [S15](#s15) | Specify all four error-diffusion recipes | [S10](#s10), [S12](#s12) | Ready | [#101](https://github.com/mia-cx/ditherette/pull/101) |
| [S16](#s16) | Specify adaptive Yliluoma mixing | [S10](#s10), [S12](#s12) | Ready | [#100](https://github.com/mia-cx/ditherette/pull/100) |
| [S17](#s17) | Complete the five-method reference processor | [S05](#s05), [S10](#s10), [S11](#s11), [S13](#s13), [S14](#s14), [S15](#s15), [S16](#s16) | Ready | [#103](https://github.com/mia-cx/ditherette/pull/103) |
| [S18](#s18) | Freeze the complete reference and enforce immutability | [S17](#s17) | Ready | [#104](https://github.com/mia-cx/ditherette/pull/104) |
| [S19](#s19) | Ship the first scalar package call with bounded memory | [S02](#s02), [S06](#s06), [S18](#s18) | Ready | [#105](https://github.com/mia-cx/ditherette/pull/105), correction [#109](https://github.com/mia-cx/ditherette/pull/109) |
| [S20](#s20) | Benchmark complete public browser calls | [S19](#s19) | Ready | [#107](https://github.com/mia-cx/ditherette/pull/107) |
| [S21](#s21) | Complete and optimize scalar bilinear and area resize | [S19](#s19), [S20](#s20) | Ready | [#110](https://github.com/mia-cx/ditherette/pull/110) |
| [S22](#s22) | Complete and optimize scalar cubic and Lanczos resize | [S19](#s19), [S20](#s20) | Ready | [#111](https://github.com/mia-cx/ditherette/pull/111) |
| [S23](#s23) | Implement and optimize scalar trilinear resize | [S21](#s21) | Ready | [#112](https://github.com/mia-cx/ditherette/pull/112) |
| [S24](#s24) | Implement packed-color direct quantization | [S19](#s19), [S20](#s20) | Ready | [#113](https://github.com/mia-cx/ditherette/pull/113) |
| [S25](#s25) | Complete weighted and perceptual matching | [S24](#s24) | Ready | [#114](https://github.com/mia-cx/ditherette/pull/114) |
| [S26](#s26) | Implement scalar Bayer and random perturbation | [S25](#s25) | Ready | [#115](https://github.com/mia-cx/ditherette/pull/115) |
| [S27](#s27) | Implement scalar blue-noise perturbation | [S26](#s26) | Ready | [#116](https://github.com/mia-cx/ditherette/pull/116) |
| [S28](#s28) | Implement all scalar diffusion modes with bounded scratch | [S25](#s25), [S26](#s26) | Ready | [#118](https://github.com/mia-cx/ditherette/pull/118) |
| [S29](#s29) | Implement and optimize scalar Yliluoma mixing | [S25](#s25), [S26](#s26) | Ready | [#117](https://github.com/mia-cx/ditherette/pull/117) |
| [S30](#s30) | Complete end-to-end process across every supported mode | [S23](#s23), [S22](#s22), [S25](#s25), [S27](#s27), [S28](#s28), [S29](#s29) | Ready | [#119](https://github.com/mia-cx/ditherette/pull/119) |
| [S31](#s31) | Memoize prepared palettes and resize plans within budget | [S30](#s30) | In progress | - |
| [S32](#s32) | Memoize shared image stages atomically | [S31](#s31) | Planning | - |
| [S33](#s33) | Add public progress and callback failure semantics | [S32](#s32) | Not started | - |
| [S34](#s34) | Implement optional threaded initialization and teardown | [S33](#s33) | Not started | - |
| [S35](#s35) | Benchmark optional resize and color row bands | [S34](#s34) | Not started | - |
| [S36](#s36) | Benchmark optional quantize and field row bands | [S34](#s34) | Not started | - |
| [S37](#s37) | Evaluate optional Yliluoma row bands | [S34](#s34) | Not started | - |
| [S38](#s38) | Integrate the complete package behind the website flag | [S30](#s30) | Ready | [#120](https://github.com/mia-cx/ditherette/pull/120) |
| [S39](#s39) | Implement website cancellation and faithful fallback | [S38](#s38), [S34](#s34) | Not started | - |
| [S40](#s40) | Run package browser, memory, and lifecycle conformance | [S35](#s35), [S36](#s36), [S37](#s37), [S39](#s39) | Not started | - |
| [S41](#s41) | Tune complete calls and assemble fresh performance evidence | [S40](#s40), [S20](#s20) | Not started | - |
| [S42](#s42) | Build reproducible tarballs and publication automation | [S34](#s34), [S40](#s40) | Not started | - |
| [S43](#s43) | Join and verify the complete unmerged implementation stack | [S41](#s41), [S42](#s42) | Not started | - |
| [S44](#s44) | Prepare the held Wasm-default rollout PR | [S43](#s43) | Not started | - |
| [S45](#s45) | Prepare the held TypeScript-retirement PR | [S44](#s44) | Not started | - |

## Available parallel work

- After S01: workspace/build ownership, reference contracts, and the benchmark guard.
- After S03: ordinary color, perceptual color, palette/alpha, and resize references, subject to the agent-slot limit.
- After S18 and package/bench foundations: production families branch from the scalar wrapper checkpoint.
- After S34: resize/color, quantize/fields, and Yliluoma threading experiments can be implemented separately; measurements remain serial.
- Website integration and packaging can advance beside independent kernel work when their own prerequisites are present.

Every multi-parent slice starts from a verified join. The index lists dependencies, not permission to cherry-pick incomplete work.

## Slice contracts

Each GitHub issue will link the new parent PRD, list its user stories and immutable prerequisites, and copy the following scoped acceptance criteria. Every agent reads the parent execution contract first.

Every production implementation issue must also include this acceptance criterion:

- [ ] Reuse already-landed production kernels and shared helpers. For genuinely missing implementations, copy frozen `spec/` into mirrored `prod/` with only mechanical wiring changes and verify the baseline before optimization. Keep `spec/` unchanged. Do not replace landed implementations or repeat their optimization work.

Benchmark obligations follow the parent PRD's bounded, filter-specific targets. No slice requires endless tuning or an artificial speedup for every scalar kernel. A measured rejected candidate satisfies an experiment obligation; final correctness and regression gates still apply.

<a id="s01"></a>

### S01. Establish the inherited port stack and validation baseline

**Phase:** Foundation. **Execution:** AFK. **Prerequisites:** None.

**What to build**

Create a clean anchor branch from the existing Rust/Wasm port, reconcile it with current main, and open the inherited-work PR before new slices stack on it.

**Acceptance criteria**

- [ ] Preserve the existing port history and root worktree changes; record the chosen main and port SHAs.
- [ ] Record focused baseline checks and distinguish existing failures from new failures.
- [ ] Open an unmerged anchor PR and establish the branch/PR/SHA ledger used by every child.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Anchor worktree, stack ledger, inherited integration only.

**Benchmark obligation:** No benchmark needed; preserve existing artifacts. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 17.

<a id="s02"></a>

### S02. Move builds and package ownership into their settled workspaces

**Phase:** Foundation. **Execution:** AFK. **Prerequisites:** [S01](#s01).

**What to build**

Establish the internal crate build/test commands and the public unscoped npm workspace, with generated scalar/threaded artifacts and a private website workspace.

**Acceptance criteria**

- [ ] Apply the approved toolchain pins, MIT license, version ownership, ignored generated output, and delegating root commands.
- [ ] Build both variants with the 2 GiB Wasm maximum into isolated workspace output directories.
- [ ] Keep public imports side-effect-free; this scaffold does not publish unfinished methods or a package.
- [ ] Preserve the existing website path until its migration slice.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Workspace manifests, toolchain/build scripts, npm package scaffold; avoid semantic kernels.

**Benchmark obligation:** Build-only; run outside another slice's benchmark phase. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 12, 13.

<a id="s03"></a>

### S03. Define reference requests, results, errors, and mode inventory

**Phase:** Foundation. **Execution:** AFK. **Prerequisites:** [S01](#s01).

**What to build**

Turn the accepted interface into shared storage types and readable reference request validation, with an exhaustive mode-to-oracle inventory.

**Acceptance criteria**

- [ ] Inventory every current and extra mode, valid metric/space pairing, public method, exported kernel, and executable adapter.
- [ ] Use existing image infrastructure; distinguish RGBA8 and indexed results with ordered palette/transparency/warning metadata.
- [ ] Define versioned request semantics, validation limits, malformed-input errors, and readable spec models for lifecycle/control behavior.
- [ ] Compile a small valid/invalid request fixture through the reference contract; no production dispatch yet.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Shared image/contracts, reference validation, mode inventory; coordinate barrel edits centrally.

**Benchmark obligation:** No performance changes. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 2, 3, 4, 5, 6, 7, 8, 9.

<a id="s04"></a>

### S04. Enforce exclusive benchmark execution across worktrees

**Phase:** Foundation. **Execution:** AFK. **Prerequisites:** [S01](#s01).

**What to build**

Make the existing benchmark entrypoints share one exclusive run guard and a coordinator protocol for quiet measurement.

**Acceptance criteria**

- [ ] One stable host-wide OS lock covers every ditherette-bench invocation and its browser/child process lifetime.
- [ ] Concurrent invocation waits or exits clearly; interruption releases the lock and cleans up owned children.
- [ ] The coordinator stops dispatching, drains implementation agents and builds/tests, then benchmarks; agents resume only after exit.
- [ ] Verify lock contention and cleanup with a short controlled fixture, not two simultaneous measurement runs.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Benchmark startup/transport and orchestration instructions.

**Benchmark obligation:** Exclusive self-test only; no optimization sampling yet. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 15, 17.

<a id="s05"></a>

### S05. Add typed three-way benchmark verification

**Phase:** Foundation. **Execution:** AFK. **Prerequisites:** [S03](#s03), [S04](#s04).

**What to build**

Extend the existing benchmark contracts to color, perturbation, quantization, fused dithering, and pipelines, reusing RGBA comparison.

**Acceptance criteria**

- [ ] Compare frozen/reference spec, accepted production, and candidate through explicit semantic identities.
- [ ] Verify indexed dimensions, palette order, alpha/warnings, indices, differing-index counts, and rendered RGBA maximum/mean/RMS error.
- [ ] Preserve raw results and reference/accepted/candidate/difference PNGs on failures.
- [ ] Keep current resize adapters working; non-exact numeric bounds never substitute for Mia's visual approval.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Benchmark API/verifier/artifacts and minimal reference subject adapters.

**Benchmark obligation:** Only exclusive verification fixtures. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 14, 15.

<a id="s06"></a>

### S06. Measure fresh accepted and candidate performance pairs

**Phase:** Foundation. **Execution:** AFK. **Prerequisites:** [S05](#s05).

**What to build**

Use current measurements of accepted code and candidate code to drive regression checks despite thermal variation.

**Acceptance criteria**

- [ ] An external coordinator runs immutable accepted and candidate benchmark executables sequentially under one shared lease, alternating order; never nest ditherette-bench processes.
- [ ] Preserve revisions, fixture content hashes, tool/machine identity, warmup, raw samples, and matched case settings.
- [ ] Keep single-call latency separate from batched throughput; fail or mark incomplete when a required baseline case is missing.
- [ ] A confirmed per-case median slowdown above 10% fails; remeasurement never silently promotes candidate code to accepted.
- [ ] Prepare both artifacts before measurement; exercise shared-lease handoff and verify the live ditherette-bench process count never exceeds one.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Benchmark runner, comparison/baseline handling, artifact identity.

**Benchmark obligation:** Exclusive paired trials with native reference fixtures. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 15.

<a id="s07"></a>

### S07. Complete sRGB, linear RGB, and YCbCr reference round trips

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S03](#s03).

**What to build**

Provide readable forward/inverse transformations and RGBA8 reconstruction for the ordinary working spaces.

**Acceptance criteria**

- [ ] Define transfer thresholds, full-range BT.601 conventions, clipping, rounding, and alpha preservation explicitly.
- [ ] Test primary/neutral/boundary colors and out-of-gamut reconstruction with manually derived vectors.
- [ ] Use triplet working colors and existing image storage; no LUTs or fast approximations in spec.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec ordinary color modules and focused tests.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 4, 5, 14.

<a id="s08"></a>

### S08. Complete Lab and LCH reference round trips

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S03](#s03).

**What to build**

Complete Oklab/OKLCH and D65 CIELAB/CIELCH forward/inverse paths.

**Acceptance criteria**

- [ ] Document units, white point, neutral hue, hue normalization, gamut handling, and final byte rounding.
- [ ] Test known conversion vectors, cylindrical seams, neutrals, and inverse reconstruction.
- [ ] Keep reference arithmetic readable; record every output-affecting convention before freeze.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec perceptual color modules and focused tests; disjoint from ordinary color work.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 4, 5, 14.

<a id="s09"></a>

### S09. Specify supplied palettes, alpha handling, and warnings

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S03](#s03).

**What to build**

Implement the reference path from a supplied ordered palette and RGBA8 alpha rules to normalized palette/output metadata.

**Acceptance criteria**

- [ ] Cover 256-entry truncation warning, first-tie order, visible/transparent filtering, transparent-only palettes, and darkest-visible fallback.
- [ ] Match current TypeScript threshold, matte, premultiplied rounding, and warning behavior using focused fixtures.
- [ ] Reject invalid palette/input forms at the request boundary and preserve caller buffers.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec palette and pipeline alpha/metadata behavior.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 6, 7, 14.

<a id="s10"></a>

### S10. Specify direct quantization for every valid metric

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S07](#s07), [S08](#s08), [S09](#s09).

**What to build**

Connect converted pixels and prepared palettes to exhaustive naive matching in every accepted metric/space pairing.

**Acceptance criteria**

- [ ] Cover Euclidean, circular-hue, CIEDE2000, CompuPhase, Rec.601, and Rec.709.
- [ ] Validate tags before allocation; preserve first-entry ties and transparent behavior.
- [ ] Use known metric vectors and tiny hand-calculated indexed outputs to test the complete reference quantize call.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec quantize/matching and focused request tests.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 4, 6, 14.

<a id="s11"></a>

### S11. Audit and complete the reference resize family

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S03](#s03).

**What to build**

Certify all resize recipes, including extra bicubic and trilinear modes, through one readable reference resize call.

**Acceptance criteria**

- [ ] Cover nearest, area, bilinear, Catmull-Rom bicubic, Lanczos2/3, and fixed/scale-aware support where specified.
- [ ] Preserve trilinear area mips, ceil-halved dimensions, max-axis LOD, bilinear levels, and final blend semantics.
- [ ] Test tiny/odd/non-square/identity/up/down/anisotropic cases with nonconstant pixels and explicit anchors.
- [ ] Complete the resize export-to-oracle inventory without copying production shortcuts into spec.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec resize modules/tests and their inventory entries.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 3, 14.

<a id="s12"></a>

### S12. Specify palette-independent adaptive placement

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S07](#s07), [S08](#s08).

**What to build**

Port the eight-neighbor placement formula with fixed, documented working-space ranges instead of palette-derived ranges.

**Acceptance criteria**

- [ ] Define ranges from the documented space domains and record their derivation and units.
- [ ] Cover sample edge clamping, radius, threshold, zero softness, and everywhere placement.
- [ ] Changing the palette cannot change the placement mask; hue/neutral cases follow the reference space and metric conventions.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec placement module and focused tests.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 14.

<a id="s13"></a>

### S13. Specify palette-free Bayer and random perturbation

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S12](#s12), [S09](#s09).

**What to build**

Implement naive RGBA8 perturbation and reference quantize-after-perturb behavior.

**Acceptance criteria**

- [ ] Cover Bayer 2/4/8/16 and the seeded random field in every reversible perturbation space.
- [ ] Derive random values from seed and global pixel index; define the exact sequence and draw count before freeze.
- [ ] Preserve alpha and define the RGBA8 reconstruction point so separable fusion has exact composition semantics.
- [ ] Test row/band boundaries, seed changes, zero strength, adaptive placement, and palette independence.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec ordered/random fields and perturb composition.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 14.

<a id="s14"></a>

### S14. Repair the blue-noise reference before freezing

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S12](#s12), [S09](#s09).

**What to build**

Replace the mislabeled transposed Bayer tile with a deterministic blue-noise threshold reference and reproducible provenance.

**Acceptance criteria**

- [ ] Confirm the existing table equals transposed Bayer; document this as a pre-freeze defect.
- [ ] Select a primary-source blue-noise generation algorithm and define reproducible numerical rank/spectral pass criteria before generating the asset; record fixed parameters/seed and content digest.
- [ ] Check rank distribution and the specified spectral criteria, preserve the numerical analysis artifact, and demonstrate palette-free RGBA8 perturbation on fixed fixtures.
- [ ] Do not add a separate public void-and-cluster mode; the generator exists only to define the included blue-noise field.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec blue-noise field, reference asset/generator, provenance and tests.

**Benchmark obligation:** Quality analysis is reference construction, not an optimization benchmark; no subjective production-loss approval implied. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 14.

<a id="s15"></a>

### S15. Specify all four error-diffusion recipes

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S10](#s10), [S12](#s12).

**What to build**

Compose naive diffusion with the selected matcher, alpha handling, adaptive placement, and serpentine order.

**Acceptance criteria**

- [ ] Cover Floyd-Steinberg, Sierra, Sierra Lite, and Atkinson tap sets and normalization.
- [ ] Preserved transparent pixels drop incoming error and emit no outgoing error.
- [ ] Exercise image edges, scan reversal, two-row taps, zero strength, and all valid matching choices.
- [ ] The oracle may use readable full-image error storage; the production ring-buffer requirement is tested later.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec error diffusion and focused composition tests.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 6, 7, 14.

<a id="s16"></a>

### S16. Specify adaptive Yliluoma mixing

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S10](#s10), [S12](#s12).

**What to build**

Complete exhaustive two-color ordered mixture search with the approved adaptive target formula.

**Acceptance criteria**

- [ ] Preserve current pair/ratio enumeration, componentwise working-coordinate interpolation, and first-tie ordering; document hue behavior explicitly.
- [ ] Use target = nearest + placementMask * (source - nearest) before mixture selection.
- [ ] Cover everywhere/adaptive placement, neutral/hue-seam fixtures, selected metrics, transparency, and stable palette indices.
- [ ] Keep the mixture search naive and readable; acceleration belongs to production.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec Yliluoma and focused composition tests.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 6, 14.

<a id="s17"></a>

### S17. Complete the five-method reference processor

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S05](#s05), [S10](#s10), [S11](#s11), [S13](#s13), [S14](#s14), [S15](#s15), [S16](#s16).

**What to build**

Compose every reference method into one testable processor contract and finish export/executable oracle coverage.

**Acceptance criteria**

- [ ] Verify process equals resize followed by ditherAndQuantize, including metadata and warnings.
- [ ] Verify separable ditherAndQuantize equals quantize(perturb) at the explicit RGBA8 intermediate.
- [ ] Complete readable lifecycle/progress/error reference behavior and tests for rejected reentry, disposal, and callback failures.
- [ ] Every public method, semantic module, kernel, and executable adapter has a named reference implementation or reference composition.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Spec pipeline/control composition and integration tests; joins all reference branches.

**Benchmark obligation:** Register final reference subjects with the benchmark infrastructure without optimizing them. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 2, 3, 4, 5, 6, 7, 8, 9, 14.

<a id="s18"></a>

### S18. Freeze the complete reference and enforce immutability

**Phase:** Reference. **Execution:** AFK. **Prerequisites:** [S17](#s17).

**What to build**

Create the immutable spec checkpoint before new production work starts.

**Acceptance criteria**

- [ ] Record a named checkpoint commit plus spec tree/content digest, reference blue-noise generator/asset, and full mode/export inventory.
- [ ] CI rejects edits to frozen spec content and semantic imports in either direction between spec and prod, including indirect helpers; rebasing cannot reset the checkpoint.
- [ ] Audit shared image dependencies so mutable infrastructure cannot silently alter frozen semantic behavior.
- [ ] Demonstrate the guard detects a controlled temporary spec change and restores the clean tree before filing the PR.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Freeze manifest/guard and validation wiring; no semantic changes after checkpoint.

**Benchmark obligation:** No optimization. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 14, 17.

<a id="s19"></a>

### S19. Ship the first scalar package call with bounded memory

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S02](#s02), [S06](#s06), [S18](#s18).

**What to build**

Implement createDitherette and a complete nearest-resize call through the public wrapper, landed optimized nearest kernel, and durable JS output.

**Acceptance criteria**

- [ ] Reuse the landed nearest kernel and its shared helpers, preserving its production dispatch and validating against the frozen reference.
- [ ] Implement isolated instance state, preflight/capacity accounting, default memory budget, fallible allocation handling, and idempotent dispose.
- [ ] Prove input preservation, durable output, structured errors, disposed rejection, and instance isolation.
- [ ] Register reference and landed production subjects, then measure the integrated call. New optimization is optional and must use the landed implementation as its comparison baseline.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Public wrapper core, private Wasm processor, nearest production path and memory allocator seam.

**Benchmark obligation:** Required exclusive native benchmark; browser boundary measurements follow immediately. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 3, 8, 9, 12.

<a id="s20"></a>

### S20. Benchmark complete public browser calls

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S19](#s19).

**What to build**

Extend ditherette-bench's browser transport from resize kernel batches to actual public package calls.

**Acceptance criteria**

- [ ] Time one public JS call per latency sample, including hashing and boundary copies; measure initialization separately.
- [ ] Distinguish empty/warm package caches from CPU-cache scrubbing and retain throughput as a separate metric.
- [ ] Use real fixture content hashes, verify outputs, preserve browser/version/artifact identity, and clean up transport children on failure.
- [ ] Provide a registration path that each later method slice extends; compare freshly built accepted/candidate artifacts.
- [ ] Provide initial TypeScript adapters for equivalent operations with matched boundaries; identify spec-only extras explicitly rather than inventing TS equivalents.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Browser benchmark transport/scripts and public-call adapters.

**Benchmark obligation:** Required exclusive browser trials; no implementation agents or builds run concurrently. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 15.

<a id="s21"></a>

### S21. Complete and optimize scalar bilinear and area resize

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S19](#s19), [S20](#s20).

**What to build**

Expose the landed optimized bilinear and area implementations through resize. Reuse their plans and shared helpers.

**Acceptance criteria**

- [ ] Each landed mode retains its optimized implementation and gains complete public-call coverage, with recorded frozen-reference checks.
- [ ] Exercise fractional edges, alpha bytes, identity, anisotropic scaling, and scale extremes.
- [ ] Measure plans/reuse or loop improvements against freshly measured accepted code and retain exact improvements.
- [ ] Preserve landed exact or bounded behavior. Archive new non-exact candidates separately and obtain Mia's approval before selecting them.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production bilinear/area modules and mode registration fragments.

**Benchmark obligation:** Required native and public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 3, 14, 15.

<a id="s22"></a>

### S22. Complete and optimize scalar cubic and Lanczos resize

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S19](#s19), [S20](#s20).

**What to build**

Expose landed bicubic and Lanczos2/3 through the package, reusing their optimized shared convolution engine and support plans.

**Acceptance criteria**

- [ ] Cover all fixed/scale-aware combinations and anchors defined by the frozen reference.
- [ ] Validate negative lobes, clipping/rounding, odd shapes, and downsampling.
- [ ] Measure integrated calls against freshly built landed production. Change contribution plans or loops only for a demonstrated remaining bottleneck.
- [ ] Preserve exact production behavior unless an existing documented visual approval covers the exact candidate.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production convolution/bicubic/Lanczos modules and registrations; disjoint from area/bilinear.

**Benchmark obligation:** Required native and public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 3, 14, 15.

<a id="s23"></a>

### S23. Implement and optimize scalar trilinear resize

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S21](#s21).

**What to build**

Add the missing trilinear package path with area mips and bilinear level blending.

**Acceptance criteria**

- [ ] Match the frozen mip/LOD/storage rounding semantics, including non-power-of-two and one-axis cases.
- [ ] Account for mip levels and temporary outputs in peak-memory preflight.
- [ ] Benchmark level reuse and buffer-lifetime improvements, preserving exact bytes.
- [ ] Expose the mode in types, dispatch, conformance, and public-call benchmark registration.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production trilinear and its mode registration.

**Benchmark obligation:** Required native and public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 3, 14, 15.

<a id="s24"></a>

### S24. Implement packed-color direct quantization

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S19](#s19), [S20](#s20).

**What to build**

Expose quantize with prepared palettes and Euclidean matching in the accepted ordinary coordinate spaces.

**Acceptance criteria**

- [ ] Production conversions use packed f32x3 with byte alpha; remove f32x4 reliance in this path.
- [ ] Copy reference palette/alpha/color/matcher behavior and return durable indexed results with exact metadata.
- [ ] Exercise the ordinary Euclidean spaces (sRGB, linear RGB, Oklab, CIELAB, YCbCr); expose cylindrical matching with its valid metrics in the next slice.
- [ ] Benchmark scalar conversions and nearest matching; retain exact measured improvements.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production color/palette/basic quantize and public quantize registration.

**Benchmark obligation:** Required conversion, quantization, and public-call benchmarks. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 4, 6, 7, 14, 15.

<a id="s25"></a>

### S25. Complete weighted and perceptual matching

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S24](#s24).

**What to build**

Expose all remaining valid weighted, circular-hue, and CIEDE2000 metric combinations, including packed OKLCH/CIELCH forward conversions.

**Acceptance criteria**

- [ ] Type tags and runtime validation agree on every supported pair.
- [ ] Match known CIEDE2000 vectors, hue seams, neutral colors, and stable tie behavior.
- [ ] Benchmark exact metric/prepared-palette acceleration separately from plain Euclidean matching.
- [ ] Update the executable mode inventory and public-call conformance matrix.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production specialized metrics, cylindrical forward conversions, and quantize registrations.

**Benchmark obligation:** Required metric and public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 4, 6, 14, 15.

<a id="s26"></a>

### S26. Implement scalar Bayer and random perturbation

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S25](#s25).

**What to build**

Expose palette-free perturb and separable ditherAndQuantize for Bayer/random modes with adaptive placement.

**Acceptance criteria**

- [ ] Copy all seven inverse conversions and placement reference behavior; expose every accepted reversible space and matching combination with preserved alpha.
- [ ] Use global pixel indexed randomness and identical seeds regardless of future tile scheduling.
- [ ] Verify exact quantize(perturb) composition, byte reconstruction, and no palette influence on fields.
- [ ] Benchmark field evaluation, conversion reuse, and placement work; keep exact winners.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production inverse conversions, ordinary fields/placement, and public perturb/fused registrations.

**Benchmark obligation:** Required field and public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 7, 14, 15.

<a id="s27"></a>

### S27. Implement scalar blue-noise perturbation

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S26](#s26).

**What to build**

Expose the corrected frozen blue-noise recipe through perturb and fused quantization.

**Acceptance criteria**

- [ ] Use the frozen asset/recipe digest and reproduce its threshold placement exactly.
- [ ] Verify arbitrary widths, repeated tile boundaries, adaptive placement, alpha, and composition.
- [ ] Benchmark exact tile addressing/storage improvements against copied baseline.
- [ ] Keep generation/provenance tooling out of the published runtime artifact unless required for execution.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production blue-noise module/asset registration.

**Benchmark obligation:** Required field and public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 14, 15.

<a id="s28"></a>

### S28. Implement all scalar diffusion modes with bounded scratch

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S25](#s25), [S26](#s26).

**What to build**

Expose all four diffusion algorithms through ditherAndQuantize using a three-row error ring.

**Acceptance criteria**

- [ ] Match the naive full-image reference for all taps, serpentine scans, transparency, placement, and matching choices.
- [ ] Prove error scratch scales with image width, not image area, and account for capacity.
- [ ] Preserve arithmetic order and error-drop behavior while benchmarking branch/loop improvements.
- [ ] All four modes work without threads and have public-call tests and benchmark cases.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production diffusion kernels/ring scratch and mode registrations.

**Benchmark obligation:** Required exactness plus native/public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 6, 7, 8, 14, 15.

<a id="s29"></a>

### S29. Implement and optimize scalar Yliluoma mixing

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S25](#s25), [S26](#s26).

**What to build**

Expose the frozen Yliluoma recipe and adaptive placement through fused quantization.

**Acceptance criteria**

- [ ] Match exhaustive pair/ratio results and stable ties for every accepted metric combination.
- [ ] Bound prepared mixture tables/memos under the memory policy; do not allocate unbounded palette cross-products.
- [ ] Benchmark exact precomputation or memoization across small and large palettes.
- [ ] Keep unapproved approximations out of the production path and preserve scalar operation.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production Yliluoma/mix preparation and registrations.

**Benchmark obligation:** Required exactness plus native/public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 6, 8, 14, 15.

<a id="s30"></a>

### S30. Complete end-to-end process across every supported mode

**Phase:** Production. **Execution:** AFK. **Prerequisites:** [S23](#s23), [S22](#s22), [S25](#s25), [S27](#s27), [S28](#s28), [S29](#s29).

**What to build**

Join the production branches and expose complete resize-to-indexed process behavior.

**Acceptance criteria**

- [ ] Every inventory mode works through its stage method and process where applicable.
- [ ] Verify composed result/metadata equality; keep intermediates in Wasm and copy only the final indexed result.
- [ ] Reject invalid requests before output work and exercise end-to-end memory plans.
- [ ] Measure full pipelines and safe exact fusion opportunities, including RGBA8 rounding barriers.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production pipeline join and final public method registration.

**Benchmark obligation:** Required exclusive full-call comparisons. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 2, 3, 4, 5, 6, 7, 14, 15.

<a id="s31"></a>

### S31. Memoize prepared palettes and resize plans within budget

**Phase:** Runtime. **Execution:** AFK. **Prerequisites:** [S30](#s30).

**What to build**

Reuse small deterministic preparation results and idle scratch across public calls.

**Acceptance criteria**

- [ ] Share the approved byte/entry budget; count capacities and drop idle scratch before LRU entries under pressure.
- [ ] Prepare keys from typed normalized settings and palette order; preserve thread-independent identity.
- [ ] Verify isolated instances, budget pressure, changed palettes/settings, and allocation failure recovery.
- [ ] Benchmark repeated settings/palette use with honest cold and warm preparation cases.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Private preparation cache/scratch lifecycle and memory accounting.

**Benchmark obligation:** Required cold/warm public-call benchmark evidence. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 8, 15.

<a id="s32"></a>

### S32. Memoize shared image stages atomically

**Phase:** Runtime. **Execution:** AFK. **Prerequisites:** [S31](#s31).

**What to build**

Cache fitted image/color/indexed stages using 256-bit content identities shared across all five methods.

**Acceptance criteria**

- [ ] Hash current input bytes and dimensions; never trust frontend IDs or retain caller-owned source buffers.
- [ ] Reuse semantic stages across methods; fused paths cache only intermediates already materialized.
- [ ] Enforce 128 entries and min(256 MiB, memoryLimitBytes/4); oversized results stay uncached.
- [ ] Publish new entries only after successful calls; test input/output mutation, failure, eviction, and exact cold/warm equality.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Private image-stage cache/identity and commit-on-success integration.

**Benchmark obligation:** Required hashing/copy-inclusive cold/warm benchmarks. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 8, 14, 15.

<a id="s33"></a>

### S33. Add public progress and callback failure semantics

**Phase:** Runtime. **Execution:** AFK. **Prerequisites:** [S32](#s32).

**What to build**

Implement optional onProgress across every method without exposing cache controls.

**Acceptance criteria**

- [ ] Emit typed stages and measurable completed/total counts, with within-stage updates throttled to 50 ms.
- [ ] Cache hits may skip stages; completion follows final output readiness and precedes successful cache publication.
- [ ] Reject callback reentry/disposal, map thrown callbacks to structured errors, and publish no new entries on failure.
- [ ] Test all methods and benchmark callback-disabled versus enabled overhead without changing output.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Private progress/lifecycle control and wrapper callback boundary.

**Benchmark obligation:** Required focused public-call overhead benchmark. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 9, 10, 14.

<a id="s34"></a>

### S34. Implement optional threaded initialization and teardown

**Phase:** Runtime. **Execution:** AFK. **Prerequisites:** [S33](#s33).

**What to build**

Load scalar or threaded artifacts through the approved threads option and manage isolated processor/thread lifetimes.

**Acceptance criteria**

- [ ] disabled is scalar; preferred falls back on capability/init failure; required returns structured capability/init errors.
- [ ] Root import stays inert and scalar initialization fetches no threaded artifacts.
- [ ] Support custom Wasm inputs and ensure budget/instance ownership agrees with actual module memory ownership.
- [ ] Verify startup failure, dispose, host termination, and worker-pool cleanup without requiring any external consumer project.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** npm loading wrapper, private threaded runtime/pool lifecycle.

**Benchmark obligation:** Initialization/lifecycle evidence; exclusive measurements for startup cost. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 8, 9, 11, 12.

<a id="s35"></a>

### S35. Benchmark optional resize and color row bands

**Phase:** Acceleration. **Execution:** AFK. **Prerequisites:** [S34](#s34).

**What to build**

Integrate and tune existing safe row-band execution for resize and color stages.

**Acceptance criteria**

- [ ] Cover global coordinates, support halos/row access, disjoint writes, and exact scalar equality.
- [ ] Account for worker scratch and all temporary capacity in memory planning.
- [ ] Use exclusive crossover sweeps to choose policy thresholds; retain scalar where threading does not help.
- [ ] Record accepted exact configurations and public-call gains rather than kernel timing alone.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production resize/color tiling policies and adapters.

**Benchmark obligation:** Required exclusive tiling and complete-call sweeps. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 3, 4, 11, 14, 15.

<a id="s36"></a>

### S36. Benchmark optional quantize and field row bands

**Phase:** Acceleration. **Execution:** AFK. **Prerequisites:** [S34](#s34).

**What to build**

Add measured row-band scheduling for direct quantization and separable fields.

**Acceptance criteria**

- [ ] Global pixel coordinates and random identities remain invariant across tile size and worker count.
- [ ] Adaptive placement reads valid neighboring source data without tile seams.
- [ ] Benchmark preparation/dispatch/copy costs and use scalar below measured crossover points.
- [ ] Error diffusion remains scalar.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production quantize/field tiling adapters and policies; disjoint from resize/color policies.

**Benchmark obligation:** Required exactness and exclusive tiling/public-call sweeps. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 4, 5, 11, 14, 15.

<a id="s37"></a>

### S37. Evaluate optional Yliluoma row bands

**Phase:** Acceleration. **Execution:** AFK. **Prerequisites:** [S34](#s34).

**What to build**

Determine whether Yliluoma benefits from exact parallel row processing with bounded shared preparation.

**Acceptance criteria**

- [ ] Compare scalar and candidate output byte-for-byte across worker counts, palette sizes, and adaptive placement.
- [ ] Account for preparation/memo duplication and shared-memory constraints.
- [ ] Keep a threaded implementation only if complete-call measurements improve; a documented scalar winner completes this slice.
- [ ] Record rejected candidates so later agents do not repeat ineffective work.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Production Yliluoma tiling/policy and benchmark cases.

**Benchmark obligation:** Required exclusive experiment; no forced parallel implementation. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 5, 11, 14, 15.

<a id="s38"></a>

### S38. Integrate the complete package behind the website flag

**Phase:** Web integration. **Execution:** AFK. **Prerequisites:** [S30](#s30).

**What to build**

Route the existing website worker through the public package while preserving its browser-owned crop, palette, preview, persistence, and export responsibilities.

**Acceptance criteria**

- [ ] Use the workspace package and translate existing settings to typed recipes.
- [ ] Keep the backend flag development-only and off by default until Mia's stability acceptance.
- [ ] Preserve user-facing controls; provide no backend selector or comparison UI.
- [ ] Verify upload/crop/settings-to-indexed-preview/export behavior in project-owned fixtures.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Website processor adapter and worker bridge; avoid package kernels.

**Benchmark obligation:** Integration timings only after exclusive scheduling; no live deployment. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 2, 7, 12, 16.

<a id="s39"></a>

### S39. Implement website cancellation and faithful fallback

**Phase:** Web integration. **Execution:** AFK. **Prerequisites:** [S38](#s38), [S34](#s34).

**What to build**

Apply the approved worker replacement, progress forwarding, stale-result rejection, and initialization fallback behavior.

**Acceptance criteria**

- [ ] Explicit cancel terminates the worker; settings use the existing slider debounce then replace active work; threaded pool cleanup is verified.
- [ ] Ignore stale results/progress immediately and keep the last valid preview.
- [ ] Fallback after load/init/capability failures only for faithfully supported requests, persist that choice for the page session, and report it.
- [ ] Keep invalid-input/memory/callback/runtime failures visible; use existing status/error presentation without new UI design.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Website request scheduling/fallback bridge and focused integration tests.

**Benchmark obligation:** Measure cancellation/startup behavior in isolated test fixtures; no benchmark overlap. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 10, 16.

<a id="s40"></a>

### S40. Run package browser, memory, and lifecycle conformance

**Phase:** Release readiness. **Execution:** AFK. **Prerequisites:** [S35](#s35), [S36](#s36), [S37](#s37), [S39](#s39).

**What to build**

Validate the integrated package in Chromium, Firefox, and WebKit with project-owned fixtures.

**Acceptance criteria**

- [ ] Cover scalar methods, custom Wasm input, supported threaded behavior, preferred fallback, and required errors.
- [ ] Exercise input preservation, output durability, separate instances, cache eviction, callback failure, disposal, and bounded repeated-use memory.
- [ ] Test supported maximum/boundary dimensions with mode-appropriate memory cases and clear preflight rejection.
- [ ] Report browser versions and absent/failed coverage honestly; no Caelestis checkout or smoke requirement.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Package/website browser conformance fixtures and CI matrix.

**Benchmark obligation:** Correctness tests run in implementation phases, never alongside benchmark measurement. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 8, 9, 10, 11, 12, 14.

<a id="s41"></a>

### S41. Tune complete calls and assemble fresh performance evidence

**Phase:** Release readiness. **Execution:** AFK. **Prerequisites:** [S40](#s40), [S20](#s20).

**What to build**

Run the agreed release matrix and improve remaining measured bottlenecks using exact production changes.

**Acceptance criteria**

- [ ] Use representative preview, common, large, and capped-output cases across direct, fields, diffusion, perceptual spaces, and extras.
- [ ] Freshly measure accepted/candidate artifacts, and equivalent TypeScript operations for the first release, under the exclusive lock with cold/warm application caches.
- [ ] Block confirmed per-case median regressions above 10%; missing/noisy required evidence is incomplete, never silently passing.
- [ ] Keep only demonstrated exact wins; preserve PNG/metric evidence for unapproved non-exact experiments without selecting them.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Final performance fixes limited to measured hotspots plus release artifact report.

**Benchmark obligation:** Required exclusive full-matrix measurements, with implementation agents drained. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 8, 14, 15.

<a id="s42"></a>

### S42. Build reproducible tarballs and publication automation

**Phase:** Release readiness. **Execution:** AFK. **Prerequisites:** [S34](#s34), [S40](#s40).

**What to build**

Prepare the unscoped package distribution and tag-driven publish workflow without publishing.

**Acceptance criteria**

- [ ] Tarball includes wrapper/types, scalar/threaded Wasm and workers, README/license, and excludes private source/benchmark payloads.
- [ ] Version alignment and approved pins are checked; clean builds and installed-tarball consumers resolve assets correctly.
- [ ] Measure raw/compressed sizes per artifact and complete tarball; record initial budget and review >10% growth.
- [ ] Validate tag/version/provenance workflow through offline/dry-run checks; do not create release tags or access publication credentials.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Package manifest/distribution/docs and CI publishing configuration.

**Benchmark obligation:** No timing benchmarks required; avoid builds during another slice's measurement. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 12, 13.

<a id="s43"></a>

### S43. Join and verify the complete unmerged implementation stack

**Phase:** Release readiness. **Execution:** AFK. **Prerequisites:** [S41](#s41), [S42](#s42).

**What to build**

Produce one integration tip containing every completed branch and a reviewable release-readiness report.

**Acceptance criteria**

- [ ] Record all issue/PR/base/head/dependency SHAs and ensure the final ancestry includes every prerequisite.
- [ ] Run required integrated checks and attach the exact artifact, conformance, performance, memory, and size evidence.
- [ ] Keep all PRs unmerged and Wasm rollout flagged; distinguish automated passes from pending human visual/size/stability approvals.
- [ ] List any unavailable gate as a blocker with reproducible commands; never declare release readiness from partial coverage.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Final join branch, release evidence and stack ledger.

**Benchmark obligation:** Reuse valid evidence from exact same artifacts; rerun only invalidated or missing measurements under the lock. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 1, 2, 12, 13, 14, 15, 16, 17.

<a id="s44"></a>

### S44. Prepare the held Wasm-default rollout PR

**Phase:** Held operational changes. **Execution:** AFK preparation; activation held. **Prerequisites:** [S43](#s43).

**What to build**

Prepare a separate PR that enables scalar Wasm by default while retaining developer-only override and temporary faithful TypeScript fallback.

**Acceptance criteria**

- [ ] Base on the verified integration tip and demonstrate the intended configuration in isolated tests.
- [ ] Mark the PR held for Mia's stability acceptance; its preparation can finish AFK.
- [ ] Do not merge, deploy, or activate the change; publish/release actions remain separate.
- [ ] Document rollback to the previous website build and the preserved fallback behavior.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Rollout configuration and focused checks; no new public controls.

**Benchmark obligation:** Reuse integration evidence unless code changes invalidate it. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 16, 17.

<a id="s45"></a>

### S45. Prepare the held TypeScript-retirement PR

**Phase:** Held operational changes. **Execution:** AFK preparation; activation held. **Prerequisites:** [S44](#s44).

**What to build**

Prepare the separate retirement diff for the old processing backend after the rollout acceptance gate.

**Acceptance criteria**

- [ ] Remove legacy processing and temporary backend selection/fallback plumbing while preserving browser TypeScript responsibilities.
- [ ] Provide the agreed retryable initialization error using existing error/retry presentation.
- [ ] Keep Rust spec comparisons permanent and validate package-only website processing.
- [ ] Mark the PR held until Mia accepts the actual initial rollout and no blocking regressions remain; do not treat AFK preparation as that acceptance.
- [ ] Open a reviewable unmerged PR against the correct parent/join; record validated head and dependency SHAs.

**Ownership:** Legacy processing removal and website error handling on its own descendant branch.

**Benchmark obligation:** Verify removal/integration; no redundant optimization benchmark unless measured behavior changes. Follow the exclusive-measurement rule from the parent PRD.

**User stories:** 16, 17.
