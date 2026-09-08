# S38 website public-package integration

Issue #80. Branch `impl/v1-s38-website` starts from validated S30 PR #119 at
`88eb79fc129662fcfc6d4554d3109855348d0316`. The immediate PR base is
`impl/v1-s30-process`. Only website mapping, worker integration, focused tests,
required workspace dependencies, and this plan belong to S38.

## Authority and current state

The approved [PRD execution contract](../docs/plans/ditherette-v1/README.md#execution-contract),
[S38 slice](../docs/plans/ditherette-v1/slices.md#s38), and issue #39 resolution
govern this integration. The coordinator's `.plans/80-reuse-inventory.md` records
the initial read-only inventory. No website implementation or test has run yet.

Work is paused at the coordinator's request. S30 PR #119 CI has a fixture omission
in protected freeze-policy tooling. Mia's authorization is required before that
separate repair. S38 must not change the policy or work around the omission.
Resume implementation only after the coordinator releases this hold.

## Existing path and intended mapping

`ProcessorWorkerPipeline.handleAsync` currently performs optional historical Wasm
resize, then TypeScript quantization. The new developer-only flag stays off by
default. When enabled, one complete public `process` call replaces that mixed path.
When disabled, existing TypeScript behavior remains unchanged. No backend selector,
comparison UI, automatic fallback, or new progress policy belongs here.

The workspace already declares `ditherette` as a dependency. Use its actual exported
types and package initialization, never historical static Wasm URLs.
Public requests contain `{source, palette, recipe}`; the recipe uses `match`.
Translate all seven website resize IDs, eight color-space IDs, nine dither IDs,
alpha controls, and placement controls explicitly. Website strength is a percentage.
Existing `wasmResizeRequest`, `bayerSizeForAlgorithm`, `supportsVectorDither`, and
`resolveMatteRgb` identify reusable mapping behavior; inspect each before extraction.
CompuPhase disables vector dithering in the current website. Weighted RGB and
cylindrical matching need explicit metric mapping, not string substitution.

Preserve ordered palette names, keys, tags, duplicates, transparency, first-256
normalization, and warnings when converting indexed package results to the existing
`ProcessedImage`. Reuse matte-selection behavior without running the TypeScript
quantizer to prepare it. Decode, source transfer, preview rendering, indexed PNG,
settings identities, and persistence remain browser-owned.

## Crop decision

The committed UI crop in `src/routes/components/ComparisonPreview.svelte` rounds
origins and sizes to integers. Worker validation requires integer width/height but
accepts finite fractional x/y. `resize.ts::clampCrop` preserves those origins and
can produce fractional edge extents. These are distinct input contracts.

The coordinator approved reusing the current clamp and packing integer rectangles
as RGBA8 for the flagged package path. If the clamped rectangle remains fractional,
return a visible unsupported-request error. Do not round it, pre-rasterize it, or
silently run another backend. The disabled-flag path retains fractional TS behavior.
Existing TS filters can read original-image neighbors outside an integer crop;
the package clamps to the already-cropped image. Record that boundary difference
and test the approved cropped-input contract without claiming universal TS parity.

## Confirmed test boundaries and atomic work

The coordinator confirmed the typed website mapper, `handleAsync`, and existing
decode/crop-to-preview/export interfaces as the test boundaries.

- [ ] Add one failing typed request-mapping test, implement its minimal mapping,
  then cover existing mode tags, crop refusal, and ordered palette metadata.
- [ ] Integrate one off-default developer flag into the worker path. Test enabled
  public Process and unchanged disabled behavior, with processing failures visible.
- [ ] Exercise project-owned decoded-upload/crop/settings fixtures through the
  actual installed package, existing render/PNG code, and persistence schemas.
- [ ] Send a clean mapping/baseline checkpoint for review before final PR work.
- [ ] Complete focused checks, rebase onto the actual S30 parent, and file a real
  unmerged, non-draft PR with auto-merge off. Return generated targets after drain.

The retained S30 tarball is approved for website-only tests:
`.worktrees/v1-s30-process/target/s30-trial-01/public/ditherette.tgz`, SHA-256
`379c733b02bc67a24500d3ae825901d17d5fa342f93d114c20761da1aa9193b2`.
Any needed generated outputs must remain local to this worktree. No compiler
target is currently created or owned. No build, test, or browser job is running.

## Scope limits

S39 owns supersession, faithful fallback, and progress forwarding. S31 owns runtime
caches in its separate worktree. Package/Rust kernels, frozen files and policy,
UI controls, publication, deployment, rollout activation, and PR merging remain
unchanged. Integration timings require separate exclusive scheduling and clearance.
