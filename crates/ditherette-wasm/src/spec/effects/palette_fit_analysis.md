# Palette-fit analysis

## Purpose

Derive a `curves` list for a `palette-fit` step from the image reaching it and the context palette,
in the step's own space. Where [recolour analysis](recolour_analysis.md) writes a private recipe,
this writes ordinary curves a caller can inspect and edit.

## Inputs and outputs

An `EffectImage` (the image reaching the step), a context supplying the palette, the step's
`space`, and its `look`. The context's working space serves the preceding steps, not the fit. Returns a `Vec<Curve>`: `[]` when there is nothing to fit.

## Algorithm / semantic rule

Sampling and the palette follow recolour analysis verbatim: visible pixels on the coarsest grid
step with at most 2¹⁸ samples, row-major, weight `alpha / 255`, alpha 0 skipped; the palette is the
visible colours among the first 256 entries with exact duplicates removed; a colour within 0.001 of
neutral counts as exactly neutral. No sample or no palette colour returns `[]`.

Coordinates are [space.md](space.md)'s `to_opponent` in the step's space (`L, u, v`; cielab is
divided by 100). Hue is `atan2(v, u)`, chroma `hypot(u, v)`, and the neutral ramp is
`min(1, c / 0.02)`. All arithmetic is `f32` in the written order.

The analysis emits up to five curves, in list order: tone, shift a, shift b, chroma gain, hue turn.
Omitted curves are absent.

1. **Tone.** Exactly recolour analysis's Tone rule (quantiles at 1/25/50/75/99%, the palette's
   lightness levels, clamping into the palette range, the quartile pull `0.5 · min(1, 4/n)`,
   knots closer than 0.001 dropped, monotone, within 1e-4 of identity kept as identity).
   Emit it, when it is not identity, as a one-input `remap` with `x = y = {lab model, lightness}`.
   Its function `tone(L)` is used again by step 6; it is the identity when not emitted.
2. **Shift** (look `fitted`). Exactly recolour's Shift rule: none when neutral is inside the hull
   (reach ≥ 0 every 5°), else half-way toward the palette centroid, at most 0.1 long. For the
   shift vector `s = (su, sv)` in opponent units, emit two one-input `adjust` curves with
   `x = {lab, lightness}` and `y = {lab, a}` or `{lab, b}`, each constant at
   `k = 0.5 + s_axis · scale`, where `scale` is `1 / 0.8` for oklab and `100 / 250` for cielab
   ([model.md](model.md) normalisation). Omit both when there is no shift.
3. **Slice reach.** `reach(l, θ)` is the support function of the palette hull's slice at lightness
   `l`: the maximum of `u cos θ + v sin θ` over palette colours with `L = l` and over every pair
   `(i, j)` with `L_i < l < L_j` at the point where segment `ij` crosses `l`. Pairs iterate `i`
   ascending then `j` ascending, with strict `>` so the first maximum wins. `l` is first clamped
   into the palette's lightness range, so the slice is never empty; a single-lightness palette
   uses its only level. The flat `reach_all(θ)` is recolour's reach: the maximum over all palette
   colours.
4. **Cells.** Columns sit at `h_j = 30j` degrees for `j = 0..11`; rows at `L_r = 0.25r` for
   `r = 0..4`. Each sample contributes `alpha weight × hue weight × lightness weight ×
   ramp(post-shift chroma)` to every cell, where the hue weight is a raised cosine of half-width
   30° on the circular distance between the sample's post-shift hue and `h_j`, and the lightness
   weight is a tent of half-width 0.25 on `|L − L_r|` with the source lightness `L` clamped to
   `[0, 1]`. Cell mass `M_jr` sums the weights; cell chroma `C_jr` is the weighted mean post-shift
   chroma. Column mass `M_j` and chroma `C_j` sum over rows; `M` is the total. A grey image
   (`M = 0`) emits neither the chroma gain nor the turn.
5. **Turns** (look `fitted`). For each column with `M_j ≥ 0.02 M`, let `t_j = 0.5 · C_j`. When
   `reach_all(h_j) < t_j`, turn toward the nearest direction within 45° whose `reach_all` reaches
   `t_j`, trying `+5, −5, +10, −10, …, +45, −45`; when none qualifies the column does not turn.
   Other columns keep turn 0. Emit a one-input `adjust` with `x = y = {lch, hue}` and 13 points:
   `[j/12, 0.5 + turn_j / 360]` for `j = 0..11`, plus `[1, 0.5 + turn_0 / 360]` repeating the seam.
   Omit when every turn is 0.
6. **Chroma gain.** Each cell's slice lightness is `l_r = tone(L_r)` and its direction
   `h_j + turn_j`. The raw gain is `reach(l_r, h_j + turn_j) / C_jr`, clamped for `fitted` to
   `[0, 1.25]`. A cell with `M_jr < 0.005 M` or `C_jr = 0` takes the mass-weighted mean gain of
   the qualifying cells in the same row, or of all qualifying cells when the row has none. Then
   `|g − 1| < 0.02` snaps to exactly 1. Emit a two-input `adjust` with `x = {lch, hue}`,
   `x2 = {lab, lightness}`, `y = {lch, chroma}`, columns `j/12`, rows `0, 0.25, …, 1`, and
   `values[r][j] = g_jr / 2` (the `curves` gain `y · (1 + (2c − 1))` makes `c = g / 2`). Omit when
   every value is exactly 0.5.

Deterministic: fixed sampling, `total_cmp` sorting, fixed iteration order.

## Why this works this way

The curve list is the recipe: transparent, editable, and already the shape the chain applies.
Splitting the hull by lightness (slice reach) lets dark saturated colours target dark palette
entries instead of the hull's flat reach, which bright colours dominate. Turning columns toward
reachable hues keeps muted regions distinct rather than letting them collapse toward grey, and the
shift step first recentres gamuts whose palettes exclude neutral, exactly as recolour does.

## Correctness invariants

- The result always validates as a `curves` list, and holds at most 5 curves.
- A transparent-only image or a palette without visible colours gives `[]`.
- A grey image gives no chroma gain and no turn.

## Edge cases

- A single-lightness palette still slices: its only level is used at every row.
- A palette whose hull excludes neutral produces a shift; one spanning neutral does not.
- Cells and columns without enough mass borrow means rather than inventing gains.

## Production obligations

Production must derive the same curves byte for byte. It may cache the list by the exact image
reaching the step, the palette, `space`, `look`, and this analysis's version.

## Non-goals

Looks other than `fitted`, and caching in the reference itself.
