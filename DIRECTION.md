# Ditherette

Ditherette turns images into palette-limited pixel art. Image preparation,
palette fitting, resizing, dithering, inspection, and export belong to one editor.
Keep that workflow clear and fast, including large images.

## Current state

- CI now delivers verified main commits to https://ditherette.mia.cx on Cloudflare,
  after fresh conformance at the exact commit. npm publication has separate gates.
- Rust/Wasm provides the normal processing path. The frozen CPU reference stays
  in `spec/`; production copies live in `prod/` and earn optimizations through benchmarks.
- The shared wgpu backend and exact channel-lookup stage exist. Normal processing
  still needs GPU integration and the remaining stages.
- Ordinary npm publication remains gated by package-size review and the recorded
  release-qualification hold. Passing website CI does not clear those gates.

## Delivery

Mia's decision on 2026-10-14 is to deploy verified changes directly to
https://ditherette.mia.cx. This is the ongoing delivery target for this work.
Merge changes after required CI and review checks, then deploy the tested source.
Website delivery proceeds independently of npm publication holds.

## Priorities

1. Make direct production delivery reliable and keep dependencies secure.
2. Reconcile release evidence against the current artifact, explain size growth,
   and resolve technical qualification blockers without weakening their checks.
3. Improve Source-preview reuse, large-output rendering, PNG export, and persistence.
4. Connect and qualify GPU stages with CPU fallback and explicit memory accounting.
5. Complete features with settled requirements, including pixel counts and graded histograms.

Search discovery should cover Wplace templates, colour conversion, dithering,
palette fitting, and quantisation. Useful public guides should lead into the editor.

## Decisions still requiring Mia

Palette-fit Revert scope, browser-tab ownership, the layer model, and automatic
preset goals remain open. New frozen rendering semantics need visual acceptance.
Size/performance exceptions and cache-policy changes require their recorded approvals.

## Delegation

Use T3-owned delegates with separate worktrees. User-facing changes, including
new UI, go to Devin Fusion Opus + SWE. Technical work goes to Codex gpt-6.1-sol.
The lead reviews and integrates results, owns this file, and controls deployment.
Serialize heavy local builds and follow the shared exclusive benchmark lease.
