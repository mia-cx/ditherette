# S38 website reuse inventory

Read-only preparation against coordinator `cdef9f75`. S38 implementation has not started.
Start only after S30 has a validated PR and its delivered head is in the new worktree.
Read issue #80, the approved PRD, and the resolution on issue #39 before implementation.

## Existing path

- `src/lib/workers/processor.worker.ts` validates requests and owns one `ProcessorWorkerPipeline`.
- `src/lib/processing/worker-pipeline.ts::handleAsync` owns source loading, sizing, resize, quantization, timing, and output metadata.
- `src/lib/processing/client.ts` owns worker creation, source transfer, settings identities, preview publication, and persistence.
- `src/lib/processing/render.ts` and `png.ts` consume the existing indexed `ProcessedImage`. Keep those responsibilities in TypeScript.

The current asynchronous pipeline calls the old optional resize wrapper, then TypeScript quantization.
`src/lib/wasm/ditherette-wasm.ts` imports historical static Wasm URLs. It does not consume the new npm package.
Its resize flag and fallback behavior do not implement the settled full-process policy.

## Mapping to inspect

Use the actual S30 package types. The public recipe contains `match`, not `matching`.
Map the existing website resize IDs, support policies, color spaces, alpha settings, and dither controls exhaustively.
Website dither strength uses percentages. Existing quantization divides by 100.
`quantize-shared.ts::supportsVectorDither` captures the current `useColorSpace` restriction for CompuPhase.
Read those semantics before choosing a recipe; do not silently replace unsupported settings.

Website palette entries carry names, keys, and tags. Preserve them by ordered index when adapting normalized package results.
Keep first-256 behavior, duplicate entries, transparency, and warnings consistent with the package contract.
`quantize-shared.ts::resolveMatteRgb` owns the current selected/disabled/missing matte behavior, but is private.
Reuse or extract that small behavior if needed. Do not invoke full TypeScript quantizer preparation just to resolve a matte.

Crop remains frontend-owned. `ComparisonPreview.svelte::normalizeCrop` rounds and clamps committed UI rectangles to integer pixels.
The low-level TypeScript resize clamp also accepts fractional rectangles, so inspect persisted and worker inputs before assuming all rectangles are integers.
Use a packed cropped RGBA8 buffer before `process`. Preserve source bytes and avoid changing UI crop controls.

## Scope and checks

Keep the full-process flag developer-only and off by default. Normal processing runs one backend.
S38 owns package mapping and worker integration. S39 owns faithful fallback, supersession, and progress forwarding after S34.
The existing explicit cancel already terminates the worker. Supersession currently sends a cancel message; preserve that finding for S39.
Package processing failures remain visible. Existing source, preview, palette, persistence, and export controls stay unchanged.

Focused checks should cover disabled-flag behavior, every website mode mapping, crop, palette metadata, and indexed preview/PNG compatibility.
Installed-package integration must use fresh S30 artifacts. Do not treat historical static Wasm or TypeScript parity as package conformance.
No UI redesign, source upload, deployment, publication, or rollout activation belongs to this slice.
