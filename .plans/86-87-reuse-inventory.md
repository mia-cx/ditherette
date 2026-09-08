# Held rollout and retirement reuse

This is a read-only inventory for S44/S45, not permission to start their implementation before S43.
Read the approved slice contracts and the final S43 report first. Both deliver unmerged, held PRs.
Neither activates deployment, publication, or Mia's acceptance gates.

## S44 rollout

The current package gate lives in `ProcessorWorkerPipeline.handleAsync`.
It requires both development mode and `VITE_DITHERETTE_WASM_PROCESS=true`.
The package initializes scalar by default through `initializePackageProcessor`.
Keep the developer override separate from public settings; no user backend selector exists.

The client already retains a page-session fallback decision and rejects stale worker messages.
Worker replacement handles cancellation. Existing server/browser fixtures exercise these paths.
Use these fixtures to prove the held default change and rollback configuration.
Keep the current narrow faithful fallback and visible unsupported-request error during this slice.

## S45 retirement

Do not delete the whole `processing/` folder. Several modules still own browser responsibilities.
Decode/source persistence, crop, palette selection, rendering, PNG export, hashes for UI identity,
and worker scheduling remain TypeScript. Preserve their current callers and behavior.

`package-adapter.ts` currently imports crop clamping from `resize.ts` and constants/types
from `quantize-shared.ts`. Extract still-live browser helpers before removing legacy processors.
`DitherPanel.svelte` also uses `bayer.ts` and `color.ts` for UI previews.
Retire processing execution without deleting the helper behavior those previews need.
Search current imports again because S41 adds benchmark TypeScript adapters.

The old processing paths include synchronous `handle`, the legacy branch of `handleAsync`,
row quantization workers, optional legacy Wasm resize loading, and their cache/metric plumbing.
Keep source loading, cancellation, progress, transfers, and package error propagation intact.
The retired fallback loader currently distinguishes initialization from processing failures.
Use the existing error/retry presentation for retryable package initialization failure; no UI redesign.

S40's public artifact preparation also compiles a TypeScript benchmark provider.
Check that dependency before removing website algorithm files. Bench-only historical comparison
inputs and live website code have different ownership. Do not silently break preparation,
claim deleted providers still exist, or replace permanent Rust spec comparisons with TypeScript.

S45 stays held until Mia accepts the actual initial rollout and its blocking regressions are resolved.
Preparing the diff and passing isolated tests does not satisfy that operational gate.
