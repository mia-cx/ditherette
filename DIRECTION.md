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
  Remaining GPU stages and physical browser/device qualification stay open.
  Port generated pointwise effects and recipe application, resize, palette matching,
  ordered/noise dithering, then Yliluoma. Error diffusion comes last because its
  recurrence constrains parallel execution.
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
