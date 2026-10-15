# #414 Wplace palette: Teal is #10AE82, Wplace uses #10AEA6

## Summary

The built-in Wplace palette lists Teal as `#10AE82`. Wplace serves `#10AEA6` (16, 174, 166).
Built-in palette keys are the upper-case hex, so the fix changes Teal's key. Saved Teal
visibility and a saved Teal matte must follow the corrected colour.

## Source check (2026-10-15)

- `https://wplace.live/` chunk `_app/immutable/chunks/DPfbjZiu.js` (sha256 `24a9d37d…`) holds
  64 entries: index 0 Transparent, then `{name:`Teal`,rgb:[16,174,166]}` at index 16.
- `hasColor(e)` returns `e<32||…extraColorsBitmap…&1<<e-32`: 31 free colours, 32 unlockable.
- Per tier, Ditherette's hex set differs from Wplace's only by Teal. Ditherette orders some
  colours differently from Wplace and uses its own names; this fix keeps both.

## Acceptance criteria

- [x] Built-in Teal is `#10AEA6` / rgb(16, 174, 166); the other 62 colours, split and order are unchanged.
- [x] Saved Teal visibility under `#10AE82` applies to corrected Teal; a stored `#10AEA6` value wins.
- [x] A Teal matte stays Teal across palette switches and storage restores, on either side of the
  correction. A palette holding the exact saved hex uses it.
- [x] The selected palette and the processing request use rgb(16, 174, 166).
- [x] Custom palettes, including ones holding `#10AE82`, keep their colours and own visibility.
- [x] Labelled before/after screenshots of the palette table, matte control and output preview.

## TODOs

- [x] Correct Teal and read saved state under its previous key (`wplace.ts`, `wplace.spec.ts`).
- [x] Resolve the matte key against the palette at read time (`wplace.ts`, `package-adapter.ts`,
  `PalettePanel.svelte`, `wplace-teal-state.spec.ts`, `PalettePanel.svelte.spec.ts`).
- [x] Correct Teal in `SPEC.md` and the recolour evaluation example.
- [x] Add a changeset.
- [x] Capture before/after media; run focused tests, check, lint and build.

## Notes

- Saved-key consumers: `ditherette:palette-enabled` (bare hex for Wplace, read through
  `paletteColorEnabled`), `ditherette:output` `matteKey`. History and fit-revert snapshots are
  in memory only; telemetry reads `paletteColorEnabled`. No IndexedDB record stores palette keys.
- A load-time matte rewrite missed palette switches: from a custom palette holding `#10AE82` to
  Wplace, the panel fell back to Black and processing to Green. The matte key now resolves where it
  meets a palette: the exact key, else its correction partner. The stored key is never rewritten,
  so switching back keeps the custom palette's exact colour.
- `SPEC.md` and `crates/ditherette-bench/examples/recolour_evaluation.rs` now list `#10AEA6`. The
  example was compile-checked only.
- Validation: server vitest for `wplace.spec.ts`, `app.spec.ts`, `wplace-teal-state.spec.ts`,
  `package-adapter.spec.ts`, `event.spec.ts`; client `PalettePanel.svelte.spec.ts`,
  `history.browser.spec.ts`; `pnpm check`, `pnpm lint`, `pnpm build:web`,
  `cargo check --example recolour_evaluation`. All exited 0.
- Captures: Chromium headless, `--disable-gpu`, CDP focus emulation, telemetry off. Desktop light
  and dark plus phone for the colour, hidden Teal and saved matte; desktop and phone light for the
  palette switch.
