# Ditherette

Ditherette turns images into palette-limited pixel art. Image preparation,
palette fitting, resizing, dithering, inspection, and export belong to one editor.
Keep that workflow clear and fast, including large images.

## Current state

- CI now delivers verified main commits to https://ditherette.mia.cx on Cloudflare,
  after fresh conformance at the exact commit. npm publication has separate gates.
- Public guides cover Wplace templates and colours, dithering, and fixed-palette
  quantisation. They lead into the editor without loading its workers or Wasm.
- Rust/Wasm provides the normal processing path. The frozen CPU reference stays
  in `spec/`; production copies live in `prod/` and earn optimizations through benchmarks.
- Indexed PNG encoding runs in a disposable worker and cancels superseded exports.
  Quantitative export benchmarks remain open.
- Saved-image persistence shares an IndexedDB connection and recovers after closure.
  Writes still wait for transaction completion; timed persistence qualification remains open.
- The dependency audit is clear. Scoped security overrides carry executable
  compatibility checks.
- The palette shows per-colour pixel counts and the total pixels to paint.
  Its tag and Edit controls remain accessible in narrow panels.
- Source previews reuse recent effects results and retain the uploaded colour
  index when effects are toggled off and on.
- Scopes inspect Source, Effects before resize and palette matching, and Output.
  Histograms compare stages and report sampled pure black and pure white clipping.
- Production Adaptive placement follows local contrast and can halo hard edges.
  The development preview compares fine and coarse perceptual contrast, then dithers
  whole qualifying areas while preserving each algorithm's normal texture. Sample size
  controls spatial scale; contrast tolerance controls the accepted colour jump.
  The editor limits sample size to 100 working pixels; Mia's useful range is
  20–60 pixels. The package API retains its wider range.
  Edge/ramp detection is visually accepted. The development preview reconstructs rounded
  regions along source-colour boundaries and feathers dither density to reduce hard cutoffs,
  isolated patches and undithered fringes. The result is visually accepted at
  https://development.ditherette.mia.cx. The first production optimization removes redundant
  distance work and is deployed there with exact scalar/threaded output checks.
  Qualified latency measurements remain pending because shared-host browser activity
  prevented a quiet benchmark window. Further performance work must preserve the accepted output.
- The GPU preview integrates exact channel-lookup effects with bounded CPU fallback.
  Full-resolution Celeste profiling covers all 27 modes, 15 matching spaces, both
  placements and three effects profiles: 2,610 terminal cases, including 177 timeouts.
  Effects execute on GPU; dithering, matching and Adaptive selection remain scalar CPU.
  Adaptive selection dominates common calls; matching and diffusion dominate ordinary
  Sierra, with expensive matching behind most timeouts. Optimize the exact selector
  and reconstruction first, then matching/diffusion; investigate Yliluoma separately.
  Shared-host timings are diagnostic, not qualified benchmarks. The reviewed
  integration and host memory bounds await merge.
  A further CPU selector candidate expands hot three-channel loops, prunes
  uncompetitive reconstruction paths and converts colours lazily in the existing
  row workspace. Exact mask, checkpoint and budget checks pass; review and
  deployment remain separate.
  The follow-up expands fixed neighbourhood visits and reuses consecutive RGB
  conversions within the call. Boundary scans also reuse overlapping column bounds
  and expand coarse probes without changing masks, checkpoints or storage.
  Further CPU gains remain worthwhile alongside GPU work.
  Mia's CPU target is Adaptive below twice the matching ordinary dither call.
  The CPU candidate reuses source-only coverage across algorithm, strength, palette and matching changes;
  invalidate it for processed-input, sample-size or contrast-tolerance changes.
  Retention uses the existing bounded cache and success-only publication.
  Warm Celeste edits meet that target in diagnostic checks; first-call analysis still needs work.
  Remaining GPU stages and physical browser/device qualification stay open.
  The next dither milestone combines palette matching, ordered/noise fields and
  the complete Adaptive selector/reconstruction on GPU. Keeping selection on CPU
  would retain the dominant delay. Preserve ordinary serpentine diffusion while
  investigating its scheduling separately; GPU Adaptive masks can also feed CPU diffusion.
  Read [the port plan](docs/plans/gpu-dithering.md) before implementing those stages.
  The GPU candidate now runs matching, independent fields and the complete Adaptive
  selector in the editor worker. One admitted source/coverage generation is reused
  across dither settings; cancellation checks run between bounded dispatch batches.
  Real Chromium and native checks pass, including 32 full-size Celeste cases.
  Diffusion retains its ordinary CPU recurrence with GPU coverage. Yliluoma, resize
  and wider package samples fall back to CPU; deployment and release gates remain separate.
  Mia now wants the remaining processing pipeline ported: every resize filter,
  Yliluoma, continuous error diffusion and remaining effect chains. Measure full
  browser calls and reduce Adaptive overhead alongside those ports; preserve
  scan order, palette indices, alpha coverage and the frozen CPU fallback.
  Keep both backends in the package. Clients choose CPU/GPU, fallback and tiling
  independently; the website consumes that public policy rather than owning a fork.
  The package candidate now includes all three artifacts, CPU row-band controls,
  strict GPU-only rejection and all GPU resize filters with halo-aware image tiles.
  Yliluoma now searches its palette-only mixture index on GPU at every matrix size
  and matching policy, including Adaptive placement. Every diffusion kernel now
  has an explicit GPU path: wavefronts for ordinary scans and continuous cooperative
  serpentine. CPU diffusion remains the default because faithful serpentine is
  substantially slower on the tested GPU. Mixed and masked effects now execute on GPU,
  including all colour-curve models and explicit palette recipes, with client-selected
  image tiling. Automatic recipe analysis stays on CPU. The combined GPU `process`
  now composes effects, recipe-v2 coverage resize and indexed stages through the same
  package facade, preserving the CPU's byte boundary and source-only coverage reuse.
- Ordinary npm publication remains gated by package-size review and the recorded
  release-qualification hold. Passing website CI does not clear those gates.

## Delivery

Mia's decision on 2026-10-14 is to deploy verified changes directly to
https://ditherette.mia.cx. This is the ongoing delivery target for this work.
Merge changes after required CI and review checks, then deploy the tested source.
Website delivery proceeds independently of npm publication holds.

GPU testing uses https://gpu.ditherette.mia.cx for the combined reviewed Adaptive,
output-memory and GPU effects stack. It has a separate Worker and uses preview
analytics while the pull requests complete their merge gates.

## Priorities

1. Make direct production delivery reliable and keep dependencies secure.
   Use the completed profile to prioritize exact Adaptive selector/reconstruction
   work, then matching/diffusion, while preserving accepted output and storage bounds.
2. Reconcile release evidence against the current artifact, explain size growth,
   and resolve technical qualification blockers without weakening their checks.
3. Improve large-output rendering, PNG export, and persistence.
4. Connect and qualify GPU stages with CPU fallback and explicit memory accounting.
5. Complete features with settled requirements.

## Decisions still requiring Mia

Palette-fit Revert scope, browser-tab ownership, the layer model, and automatic
preset goals remain open. New frozen rendering semantics need visual acceptance.
Size/performance exceptions and cache-policy changes require their recorded approvals.

## Delegation

Use T3-owned delegates with separate worktrees. User-facing changes, including
new UI, go to Devin Fusion Opus + SWE. Technical work goes to Codex gpt-6.1-sol.
The lead reviews and integrates results, owns this file, and controls deployment.
Serialize heavy local builds and follow the shared exclusive benchmark lease.
