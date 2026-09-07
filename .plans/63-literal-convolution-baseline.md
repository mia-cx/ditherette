# S22 literal convolution baseline

Issue #63 starts from `711c7aec61587b45a91c2e404583161edb1e0de9` on `impl/v1-s22-convolution`.
The immediate PR base is `impl/v1-s20-browser-bench`; the coordinator owns PR filing and aggregate progress.

## Bounded deliverable

Copy frozen bicubic, Lanczos, and convolution into canonical production paths with mechanical import substitutions only.
Preserve inherited optimizations under explicit candidate paths. Keep their tests and legacy callers pointed at those candidates.
Shared coordinates/sample copies must remain byte-identical to the frozen files. Existing common alignment stays unchanged.

## TODOs

- [~] Commit literal kernels, preserved candidates, mechanical caller redirects, content manifests, and exact native conformance.
- [ ] Record final Wasm compilation, formatting, and trusted freeze validation; push the baseline and drain all owned processes.

## Validation scope

Compare independently executed frozen and production functions for both support policies, all nine anchors, odd and single-axis shapes,
downsampling, padded source/output storage, integer and floating formats, negative lobes, clipping, and rounding.
Conformance must not invoke benchmark timing code. A recorded literal commit is historical evidence, not a permanent ban on later production optimization.

Public processor/package integration, benchmark registration of canonical baselines, bounded optimization, native/public timing,
and the complete S22 PR remain coordinator-owned follow-up work. No optimization or measurement is authorized in this phase.
