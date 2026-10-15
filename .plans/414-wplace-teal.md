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

- [ ] Built-in Teal is `#10AEA6` / rgb(16, 174, 166); the other 62 colours, split and order are unchanged.
- [ ] Saved Teal visibility under `#10AE82` applies to corrected Teal; a stored `#10AEA6` value wins.
- [ ] A saved matte of `#10AE82` follows to `#10AEA6` unless the active palette has `#10AE82` itself.
- [ ] The selected palette and the processing request use rgb(16, 174, 166).
- [ ] Custom palettes, including ones holding `#10AE82`, keep their colours and own visibility.
- [ ] Labelled before/after screenshots of the palette table, matte control and output preview.

## TODOs

- [ ] Correct Teal and read saved state under its previous key (`wplace.ts`, `wplace.spec.ts`).
- [ ] Carry a saved Teal matte to the corrected key on load (`app.ts`, migration spec).
- [ ] Add a changeset.
- [ ] Capture before/after media; run focused tests, check, lint and build.

## Notes

- Saved-key consumers: `ditherette:palette-enabled` (bare hex for Wplace, read through
  `paletteColorEnabled`), `ditherette:output` `matteKey`. History and fit-revert snapshots are
  in memory only; telemetry reads `paletteColorEnabled`. No IndexedDB record stores palette keys.
- Without the matte step, the palette panel's fallback finds no previous colour and picks Black,
  and processing warns and uses the colour nearest to the old Teal (Green).
- Out of scope, reported to the parent: `SPEC.md` lists `#10ae82` in the Wplace free palette,
  and `crates/ditherette-bench/examples/recolour_evaluation.rs` hard-codes `0x10AE82`.
