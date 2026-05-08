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

## Scope

### In scope

- Validate runtime data at trust boundaries:
  - IndexedDB loads
  - worker ingress/egress
  - PNG export inputs
  - restored source/processed image records
- Make async processing ownership explicit where stale work can update state after newer work starts.
- Centralize alpha/matte helpers and use one model across resize/quantization paths.
- Replace loose processing identifiers with typed registries/guards where they affect processing behavior.
- Fix targeted parsing/validation correctness bugs in metadata, PNG, resize, palette matching, and source restore flows.
- Remove small dead exports in processing modules when touched.

### Out of scope / deferred work

- UI panel restructuring already landed in #11.
- Export readiness/hash and persistence UX semantics remain #7 unless a helper must be shared by processing code now.
- Product expansion around color grading/scopes remains #5.
- Large performance rewrites are out of scope unless required to fix a correctness issue.

## Findings and intended disposition

| #6 finding | Disposition |
| --- | --- |
| Async ownership/commit model inconsistent | In scope. Add request/source tokens, terminate/settle workers predictably, and resolve DB writes on transaction completion. |
| Runtime boundary validation too shallow | In scope. Add focused validators/schemas for persisted records, worker messages, and PNG export data. |
| Alpha/matte duplicated and inconsistent | In scope. Centralize matte/premultiply/unpremultiply helpers and reuse from resize/quantize paths. |
| Algorithm identifiers loose | In scope where processing dispatch is affected. Use typed registries/guards/exhaustive dispatch. |
| `enabledPalette` ambiguous API | In scope if it affects processing inputs; prefer explicit custom-palette enabled-state contract. |
| `paletteEnabledKey` delimiter collision | In scope as processing/state key correctness. Use collision-safe tuple serialization or reject impossible delimiter input. |
| `paletteIndex` `Uint8Array` wrap | In scope. Validate palette size or widen index storage deliberately. |
| Weighted RGB vector mismatch | In scope. Ensure nearest-color matching and vectorization agree or avoid vector path for weighted modes. |
| DB multi-transaction clear / canvas fallback / settings hash | In scope, but keep fixes minimal and boundary-oriented. |
| Metadata malformed PNG/JPEG parsing | In scope. Add chunk/segment length guards and tests. |
| PNG encoder validation/copying | Validate inputs now; optimize copies only if natural. |
| Quantize side effects/magic constants | In scope if touched by alpha/matte cleanup; name constants and return warnings explicitly. |
| Unused `drawImageDataToCanvas` / `setSourceRecord` export | Remove if still unused. |
| Resize unchecked output/crop inputs | In scope. Validate finite positive dimensions/crop at entry. |
| Restore scheduling after cache hit | In scope. Do not schedule unnecessary processing after valid restore. |
| Wplace dependency leak | In scope. Processing should use `PaletteColor.kind === 'transparent'`, not Wplace constants, where possible. |

## Work order

### 1. Boundary validators and tests first

Create a small validation layer for processing records/messages without overbuilding a schema framework.

Targets:

- `SourceImageRecord`
- `ProcessedImage`
- worker request/response shape where messages cross worker boundaries
- PNG export input invariants:
  - positive integer dimensions
  - `indices.length === width * height`
  - palette length 1–256
  - every index `< palette.length`

Add unit tests around the validators and the currently failing edge cases where practical.

### 2. Async ownership and persistence commits

Trace source upload → decode → persist → process worker → restore.

Fix root contracts:

- Source/request generation tokens prevent stale async completion from committing state.
- Clear timers before source guards where stale timers can fire.
- Workers are terminated on settle/error and cannot post late results into current state.
- IndexedDB writes resolve on transaction completion, not request success.
- `clearPersistedImages` clears source and processed records in one transaction.
- Restore failures clear bad persisted records and throw/report with context.

### 3. Processing identifier registries

Move or derive processing dispatch tables from typed records:

- Bayer size lookup
- error diffusion kernels
- resize algorithm dispatch

Use `Object.hasOwn`/type guards for external or persisted IDs. Prefer exhaustive dispatch for internal unions.

### 4. Alpha/matte correctness

Centralize helpers for:

- blend over matte
- premultiply
- unpremultiply
- alpha-aware sampling/accumulation where resize currently filters RGB independently from alpha

Use helpers consistently in resize and quantization paths. Keep transparent-color detection based on `PaletteColor.kind` instead of Wplace-specific keys where possible.

### 5. Targeted parser/export correctness

Fix concrete parser/export issues:

- PNG metadata requires IHDR length/type before dimension reads.
- JPEG SOF parsing requires enough segment bytes before dimension reads.
- PNG export validates dimensions, palette, indices, and transparent index before encoding.
- Palette matching cannot silently wrap indices above 255.
- Weighted RGB modes either use correct scaled vectors or non-vector distance paths.

### 6. Cleanup pass

Remove or make private unused processing exports if still unused:

- `drawImageDataToCanvas`
- `setSourceRecord`

Name unexplained constants such as dither noise scale and scale-factor lower bound.

### 7. Validation and PR polish

Run:

```bash
pnpm check
pnpm exec eslint src/lib/processing src/lib/workers src/lib/palette
pnpm test:unit -- --run
```

Manual smoke:

- Upload a valid image and confirm processing completes.
- Change resize/dither/color-space settings and confirm no stale result wins after rapid changes.
- Reload and confirm valid persisted source/processed state restores without unnecessary recompute when cache is valid.
- Export PNG after processing and confirm malformed internal states are rejected loudly in tests.

## Acceptance criteria

- Runtime validators reject malformed persisted records, worker messages, and PNG export data at module boundaries.
- Stale async decode/process/persist work cannot update current app state after a newer source/request wins.
- IndexedDB write helpers resolve only after transaction completion.
- Alpha/matte behavior is represented by shared helpers and used consistently by quantize/resize paths touched in this slice.
- Processing dispatch for dither/resize identifiers is typed and fails loudly for unsupported external values.
- Metadata parsers reject malformed/truncated PNG/JPEG headers instead of reading bogus dimensions.
- Palette indexing cannot silently wrap above 255 entries.
- Wplace-specific transparency checks are removed from generic processing code where possible.
- `pnpm check`, targeted ESLint, and unit tests pass.

## Draft PR body seed

This PR fixes the processing pipeline findings from #6 after the UI structure refactor landed. It focuses on correctness at runtime boundaries, async ownership, alpha/matte consistency, typed dispatch, and parser/export validation.

Refs #6, #12
