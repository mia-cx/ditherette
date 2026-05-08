# Processing Pipeline Correctness Slice

## Goal

Address the processing-domain review findings from #6 by tightening runtime boundaries, removing preview/production drift where it affects output correctness, and centralizing processing helpers that currently have duplicated or implicit contracts. This slice should make the browser processing pipeline trustworthy before the later state/export hardening pass (#7).

## Linked issue

- Primary issue: #6 — Code review: processing pipeline findings
- Parent roadmap: #12
- Follows: #11 — UI structure refactor
- Blocks: #7 — State/export correctness hardening

## Worktree setup

```bash
git worktree add .worktrees/code-review-processing-fixes -b fix/processing-review-findings main
cd .worktrees/code-review-processing-fixes
```

## Issue-comment context incorporated

Issue #6 includes follow-up architectural and performance guidance. This plan treats those comments as part of the spec:

- This is an **architecture hardening/refactor slice**, not a grab bag of local patches.
- Prefer **fewer, deeper modules** with crisp responsibilities:
  - persistence owns IndexedDB reads/writes and transaction completion semantics only
  - image decode/capability owns browser image decode feature detection and blob decoding
  - processing lifecycle owns source/request tokens, cancellation, worker cleanup, progress, and stale-result suppression
  - runtime schemas own trust-boundary validation and normalization
  - alpha/compositing owns matte, premultiply, unpremultiply, and transparent handling across resize/quantize/render/export
- Use typed data shaping at boundaries: normalize/validate DB records, worker messages, and processed images once, then pass trusted types internally.
- Keep worker ownership first-class. Heavy work stays off the main thread, and lifecycle APIs should not assume one disposable worker per job if that would block a future persistent worker.
- Design for instrumentation hooks now without building a benchmarking system: decode, resize, palette matcher construction, quantize/dither, PNG encode, IDB save/restore, worker transfer, and preview render should have clear stage boundaries where timing can plug in later.
- Separate cacheable stages conceptually. Do not bake in “every setting change reruns everything”; leave room for future source/crop/resize/palette/color-space/dither/alpha cache tiers.
- Preserve memory/transfer-conscious boundaries. Validators should check cheap invariants useful for future budgets: dimensions, output pixel count, index buffer length, palette length, settings hash, and source identity. PNG hardening should avoid making encoding more dependent on extra main-thread copies.
- Keep preview vs final quality modes possible in types/settings, even if this slice does not implement quality modes.

## Scope

### In scope

- Validate and normalize runtime data at trust boundaries:
  - IndexedDB loads
  - worker ingress/egress
  - PNG export inputs
  - restored source/processed image records
- Split current mixed-responsibility processing modules only where it creates deeper boundaries:
  - persistence vs decode/capability vs lifecycle vs schema vs compositing
- Make async processing ownership explicit where stale work can update state after newer work starts.
- Preserve a worker-first lifecycle shape that can evolve into a persistent worker later.
- Centralize alpha/matte helpers and use one model across resize/quantization paths.
- Replace loose processing identifiers with typed registries/guards where they affect processing behavior.
- Fix targeted parsing/validation correctness bugs in metadata, PNG, resize, palette matching, and source restore flows.
- Add narrow lifecycle/stage seams for future instrumentation without implementing full benchmarking.
- Keep future cache tiers and preview/final quality modes possible by not collapsing all processing state into one monolithic request contract.
- Remove small dead exports in processing modules when touched.

### Out of scope / deferred work

- UI panel restructuring already landed in #11.
- Export readiness/hash and persistence UX semantics remain #7 unless a helper must be shared by processing code now.
- Product expansion around color grading/scopes remains #5.
- Large performance rewrites are out of scope unless required to fix a correctness issue.
- Full benchmarking, worker pools, persistent-worker migration, cache-tier implementation, and quality-mode product work are deferred. This slice should shape boundaries so those later efforts are easier.

## Findings and intended disposition

| #6 finding                                                   | Disposition                                                                                                                 |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Async ownership/commit model inconsistent                    | In scope. Add request/source tokens, terminate/settle workers predictably, and resolve DB writes on transaction completion. |
| Runtime boundary validation too shallow                      | In scope. Add focused validators/schemas for persisted records, worker messages, and PNG export data.                       |
| Alpha/matte duplicated and inconsistent                      | In scope. Centralize matte/premultiply/unpremultiply helpers and reuse from resize/quantize paths.                          |
| Algorithm identifiers loose                                  | In scope where processing dispatch is affected. Use typed registries/guards/exhaustive dispatch.                            |
| `enabledPalette` ambiguous API                               | In scope if it affects processing inputs; prefer explicit custom-palette enabled-state contract.                            |
| `paletteEnabledKey` delimiter collision                      | In scope as processing/state key correctness. Use collision-safe tuple serialization or reject impossible delimiter input.  |
| `paletteIndex` `Uint8Array` wrap                             | In scope. Validate palette size or widen index storage deliberately.                                                        |
| Weighted RGB vector mismatch                                 | In scope. Ensure nearest-color matching and vectorization agree or avoid vector path for weighted modes.                    |
| DB multi-transaction clear / canvas fallback / settings hash | In scope, but keep fixes minimal and boundary-oriented.                                                                     |
| Metadata malformed PNG/JPEG parsing                          | In scope. Add chunk/segment length guards and tests.                                                                        |
| PNG encoder validation/copying                               | Validate inputs now; optimize copies only if natural.                                                                       |
| Quantize side effects/magic constants                        | In scope if touched by alpha/matte cleanup; name constants and return warnings explicitly.                                  |
| Unused `drawImageDataToCanvas` / `setSourceRecord` export    | Remove if still unused.                                                                                                     |
| Resize unchecked output/crop inputs                          | In scope. Validate finite positive dimensions/crop at entry.                                                                |
| Restore scheduling after cache hit                           | In scope. Do not schedule unnecessary processing after valid restore.                                                       |
| Wplace dependency leak                                       | In scope. Processing should use `PaletteColor.kind === 'transparent'`, not Wplace constants, where possible.                |

## Work order

### 1. Establish deeper processing module boundaries

Before fixing individual symptoms, separate responsibilities enough that later fixes land in the right home:

- persistence module: IndexedDB open/read/write/delete and transaction completion semantics
- image decode/capability module: accepted image validation, capability detection, blob/image decode
- schema module: runtime validation and normalization for trust-boundary records/messages
- lifecycle module or lifecycle-owned functions: source/request tokens, cancellation, worker ownership, progress, stale suppression
- compositing module: alpha/matte/premultiply/unpremultiply helpers

Do not split for aesthetics. Split only when a module boundary gives one owner for a contract reviewers flagged as ambiguous.

### 2. Boundary validators and tests

Create a small validation layer for processing records/messages without overbuilding a schema framework.

Targets:

- `SourceImageRecord`
- `ProcessedImage`
- `WorkerRequest` / worker response shape where messages cross worker boundaries
- source/settings identity fields needed to suppress stale work and support future cache tiers
- cheap performance/memory invariants:
  - positive dimensions
  - output pixel count within project limits
  - index buffer length
  - palette length
  - settings hash/source identity shape
- PNG export input invariants:
  - positive integer dimensions
  - `indices.length === width * height`
  - palette length 1–256
  - every index `< palette.length`

Add unit tests around the validators and the currently failing edge cases where practical.

### 3. Async ownership and persistence commits

Trace source upload → decode → persist → process worker → restore.

Fix root contracts:

- Source/request generation tokens prevent stale async completion from committing state.
- Clear timers before source guards where stale timers can fire.
- Worker ownership is explicit: cancellation, settle/error cleanup, and late-message rejection are one contract.
- Keep the API compatible with a future persistent worker; do not require a new worker per job in public contracts.
- IndexedDB writes resolve on transaction completion, not request success.
- `clearPersistedImages` clears source and processed records in one transaction.
- Restore failures clear bad persisted records and throw/report with context.
- No-op guards avoid churn where setting updates would schedule identical processing.

### 4. Processing identifier registries

Move or derive processing dispatch tables from typed records:

- Bayer size lookup
- error diffusion kernels
- resize algorithm dispatch

Use `Object.hasOwn`/type guards for external or persisted IDs. Prefer exhaustive dispatch for internal unions.

### 5. Alpha/matte correctness

Centralize helpers for:

- blend over matte
- premultiply
- unpremultiply
- alpha-aware sampling/accumulation where resize currently filters RGB independently from alpha

Use helpers consistently in resize and quantization paths. Keep transparent-color detection based on `PaletteColor.kind` instead of Wplace-specific keys where possible. The helper boundary should also be a natural future instrumentation point for alpha-heavy processing costs.

### 6. Targeted parser/export correctness

Fix concrete parser/export issues:

- PNG metadata requires IHDR length/type before dimension reads.
- JPEG SOF parsing requires enough segment bytes before dimension reads.
- PNG export validates dimensions, palette, indices, and transparent index before encoding.
- PNG export validation/encoding avoids adding assumptions that force extra main-thread full-buffer copies later.
- Palette matching cannot silently wrap indices above 255.
- Weighted RGB modes either use correct scaled vectors or non-vector distance paths.

### 7. Cleanup pass

Remove or make private unused processing exports if still unused:

- `drawImageDataToCanvas`
- `setSourceRecord`

Name unexplained constants such as dither noise scale and scale-factor lower bound.

### 8. Validation and PR polish

Run:

```bash
pnpm check
pnpm exec eslint src/lib/processing src/lib/workers src/lib/palette
pnpm test:unit -- --run
```

Manual smoke:

- Upload a valid image and confirm processing completes.
- Start overlapping uploads/setting changes and confirm no stale decode/process result wins.
- Cancel/replace work and confirm worker cleanup/rejection of late messages.
- Reload and confirm valid persisted source/processed state restores without unnecessary recompute when cache is valid.
- Simulate invalid persisted data and confirm it is cleared/rejected with context.
- Process transparent images and inspect edge behavior for matte/premultiplied handling.
- Export indexed PNG after processing and confirm malformed internal states are rejected loudly in tests.

## Acceptance criteria

- Runtime validators reject malformed persisted records, worker messages, and PNG export data at module boundaries.
- Processing responsibilities are owned by deeper modules: persistence, decode/capability, schemas, lifecycle, and compositing are not tangled in one DB/source helper module.
- Stale async decode/process/persist work cannot update current app state after a newer source/request wins.
- Worker lifecycle cleanup and late-message rejection are explicit and compatible with a future persistent-worker model.
- IndexedDB write helpers resolve only after transaction completion.
- Alpha/matte behavior is represented by shared helpers and used consistently by quantize/resize paths touched in this slice.
- Processing dispatch for dither/resize identifiers is typed and fails loudly for unsupported external values.
- Metadata parsers reject malformed/truncated PNG/JPEG headers instead of reading bogus dimensions.
- Palette indexing cannot silently wrap above 255 entries.
- Wplace-specific transparency checks are removed from generic processing code where possible.
- Stage boundaries remain easy to instrument later without adding full benchmarking now.
- The request/data shapes leave room for future cache tiers and preview/final quality modes.
- `pnpm check`, targeted ESLint, and unit tests pass.

## Draft PR body seed

This PR fixes the processing pipeline findings from #6 after the UI structure refactor landed. It incorporates the architectural/performance context from #6 comments by hardening deeper module boundaries for persistence, decode/capability, schemas, lifecycle, and compositing while preserving future instrumentation, cache-tier, and persistent-worker paths.

Refs #6, #12
