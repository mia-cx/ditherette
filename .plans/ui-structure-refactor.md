# UI Structure Refactor Slice

## Goal

Address the UI-domain review findings from #4 as an informed refactor, not a pile of one-off fixes. This slice focuses on component boundaries, typed option registries, wrapper APIs, accessibility, and moving UI-only helpers out of large Svelte files. It intentionally does **not** solve production pipeline correctness; that gets its own pipeline slice after UI structure is cleaner.

## Scope

### In scope

- Split oversized UI panels where it reduces state/control-flow coupling.
- Replace fragile UI wrapper behavior with explicit props.
- Type and split option registries currently collected in `sample-data.ts`.
- Remove dead sample palette exports.
- Fix low-risk accessibility and class-list review findings while touching components.
- Extract UI/domain helper code when the component is currently carrying unrelated responsibilities.
- Keep current visual behavior unless a review finding requires otherwise.

### Out of scope / deferred work

- **Reunifying dither preview with the production quantizer** → deferred to #6. This slice may isolate the preview behind cleaner component/helper boundaries, but it should not change production quantization behavior or make previews call the production pipeline yet.
- **Processing-cache/export hash redesign** → deferred to #7, after #6. Export freshness spans processing cache and persistence semantics, so it should be fixed after the pipeline layer is cleaned up.
- **Large color grading/scopes work** → deferred to #5. Histograms, waveforms, vectorscopes, curves, wheels, levels, and Lumetri-style grading are product-expansion work, not part of this structural cleanup.
- **Full workspace/dockable editor UI** → planned but not filed as a dedicated implementation issue yet. This slice should make panels easier to move later, but it should not add docking, floating windows, or a Photoshop/Photopea-style shell.
- **Deep PalettePanel behavior hardening** → only low-risk structure/accessibility fixes belong here. Destructive/import correctness fixes that require state-flow changes should be split into a follow-up issue if they grow beyond safe extraction.

## Raw #4 findings and disposition

| #4 finding | Disposition in this slice |
| --- | --- |
| 1. Dither preview logic drifted from production quantization | **Partial / defer core fix to #6.** This slice may isolate preview code into `DitherPreviewCanvas` or a helper, but production/preview behavior unification belongs to #6. |
| 2. Panel state ownership is inconsistent and can go stale | **Mostly already addressed by state ownership prep (#9).** Avoid reintroducing local mirror/write-back patterns while splitting components. |
| 3. Async clear/restore flows can silently fail or update after unmount | **Already addressed by state ownership prep (#9) unless validation proves otherwise.** Not a target of this slice. |
| 4. Numeric parsing and clamping are inconsistent | **Partial / opportunistic.** Fix low-risk UI-side parsing while touching components; broader numeric helper consolidation can follow if it crosses pipeline/state boundaries. |
| 5. Option/sample data is dead, mutable, and weakly typed | **In scope.** Split `sample-data.ts`, delete dead sample palette exports, and type readonly option registries with existing ID unions. |
| 6. Icon/controlled controls can be rendered inaccessible or inert | **In scope.** Add missing accessible names and tighten controlled/icon component APIs where low-risk. |
| 7. Export readiness hash duplicates processing cache logic | **Deferred to #7.** This crosses processing/cache/persistence boundaries and should not be solved in the UI structure slice. |
| 8. Input wrapper has unsafe file-input typing/binding | **In scope if still present after #9.** Fix wrapper typing/binding and class duplication as part of UI wrapper cleanup. |
| 9. Select wrapper internals contain fragile class-driven behavior/noise | **In scope.** Replace class-string inspection with explicit props and remove duplicate utilities. |
| 10. Tailwind dynamic class interpolation may be missed by static extraction | **In scope.** Replace dynamic class fragments with explicit conditional class strings. |
| 11. Comparison preview pointer/fit cleanup has edge cases | **Partial / opportunistic.** Fix small cleanup if isolated; deeper preview numeric/view behavior should be grouped with numeric helper work if it grows. |
| 12. Palette panel destructive/import flows have state-consistency bugs | **Partial.** Only address when naturally covered by a safe extraction; otherwise split follow-up under #4 or a child issue. |
| 13. Export filename hides nullable processed-image dependency | **In scope if low-risk.** This is a local UI cleanup unless it touches export/cache semantics, in which case defer to #7. |

## Work order

### 1. Option registry cleanup

- Replace `sample-data.ts` with smaller domain files:
  - `color-space-options.ts`
  - `dither-options.ts`
  - `output-options.ts` or equivalent for resize/alpha/placement modes
- Delete unused `SAMPLE_PALETTE`, `SwatchKind`, and `Swatch` exports, or move them to fixtures if anything still needs them.
- Add shared option types where useful:
  - `LabeledOption<TId extends string>`
  - richer `DitherOption` / `ColorSpaceOption` types
- Type option arrays with existing processing/store unions:
  - `ColorSpaceId`
  - `DitherId`
  - `ResizeId`
  - `AlphaMode`
  - `DitherPlacement`
- Export readonly arrays via `as const satisfies readonly ...[]` where practical.

### 2. Select wrapper cleanup

- Replace `className?.includes('overflow-hidden')` in `SelectContent` with an explicit prop, e.g. `viewportFlex` or `viewportClass`.
- Update rich selects to use that prop instead of relying on class-string inspection.
- Remove duplicated utilities from select trigger/item classes.
- Preserve the dither selector behavior: fixed-height dropdown, list-only scrolling, filters fixed, `preventScroll={false}` where outside tuning must remain interactive.

### 3. Dither panel decomposition

Split `DitherPanel.svelte` only along boundaries that reduce coupling:

- `DitherAlgorithmSelect.svelte`
  - trigger card
  - search/filter UI
  - mobile filter sheet / desktop filter rail
  - option cards
- `DitherPreviewCanvas.svelte` or action/helper module
  - current preview canvas behavior isolated from selector markup
  - no production quantizer rewrite in this slice
- `AdaptivePlacementControls.svelte`
  - placement mode, radius, threshold, softness
- `DitherMathControls.svelte` / `DitherToggles.svelte`
  - dither-in-selected-space and serpentine toggles

Keep shared constants/helpers close to their owning concern. Do not create generic abstractions unless there are multiple real call sites.

### 4. Palette panel decomposition plan + first extraction

`PalettePanel` is large enough that a full split may be its own follow-up. In this slice, do one safe extraction that proves the direction without destabilizing palette behavior.

Candidate extraction order:

1. `PaletteToolbar.svelte` for palette-level actions and view toggle.
2. `PaletteSwatchRow.svelte` / `PaletteSwatchGridItem.svelte` if row/grid markup is the clearest boundary.
3. `PaletteImportDialog.svelte` if dialog state is easy to isolate.

Fix opportunistic review findings only where the touched boundary makes it natural:

- remove boolean-trap `exportPalette(true)` if touching export actions
- reset pending import state on cancel if touching import dialog
- avoid changing delete semantics unless implementing that flow fully

### 5. Accessibility and dynamic-class cleanup

- Add missing accessible names, including small-screen Clear button.
- Make controlled/icon components harder to misuse where low-risk.
- Replace dynamic Tailwind interpolation such as `gap-{compact ? '3' : '4'}` with explicit conditional class strings.

### 6. Validation and PR polish

Run:

```bash
pnpm check
pnpm exec eslint src/routes/+page.svelte src/routes/components src/lib/components/ui/select src/lib/components/ui/input
pnpm test:unit -- --run
```

Manually smoke the affected UI:

- Dither algorithm dropdown open/close/search/filter.
- Strength slider remains usable while dither dropdown is open.
- Mobile filter sheet and desktop filter rail.
- Color-space select with rendered math.
- Palette panel basic actions still render.
- AppBar clear/open buttons accessible and functional.

## Deliverables

- Smaller, single-concern UI components for the dither controls.
- Typed, readonly option registries by domain.
- Explicit select wrapper layout API.
- Low-risk accessibility and class cleanup.
- A PR that closes or substantially reduces the UI-structure portions of #4.

## Follow-up slices

- Pipeline correctness: share dither preview/production quantization helpers and export hash logic.
- Palette behavior hardening: transparent bulk delete, failure-aware destructive flows, import cancel cleanup.
- Workspace UI: larger editor shell and panel layout.
