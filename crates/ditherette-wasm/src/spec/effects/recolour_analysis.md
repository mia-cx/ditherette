# Recolour analysis

## Purpose

Derive a recolouring recipe from an image and an enabled palette together. The same image gets a different treatment for a different palette, and vice versa.

## Inputs and outputs

An `EffectImage` (the image reaching the recolour step) and a context with palette and working space. Returns a `RecolourRecipe` in that space.

## Algorithm / semantic rule

Coordinates are [space.md](space.md)'s lightness–opponent form in the context space. Constants are in `recolour_analysis.rs`.

**Samples.** Read visible pixels on the coarsest grid step with at most 2¹⁸ samples, row-major. Alpha 0 is skipped; others weigh `alpha / 255`.
**Palette.** Visible colours among the first 256 entries, exact duplicates removed.
In both samples and palette, a colour within 0.001 of neutral counts as exactly neutral, since byte greys carry `f32` residue in perceptual spaces.
With no sample or no palette colour, return the identity recipe.

**Reach.** For a hue direction `θ`, the palette's reach is `max_j (u_j cos θ + v_j sin θ)`: the support function of its convex hull.
Every point of the hull is an average of palette colours, which is what dithering shows from a distance. Reach says how saturated a colour of hue `θ` the palette can mix.

**Tone.** Take weighted lightness quantiles at 1%, 25%, 50%, 75%, and 99%, and the palette's distinct lightness levels.
If either spans less than 0.001, keep the identity curve.
Clamp the 1%–99% range into the palette's lightness range, and map lightness linearly between the two, extended to 0 and 1 and clamped to the palette range. Tones the palette already covers stay put.
Palettes with `n` lightness levels also pull each quartile toward the palette's own level at the same rank, by `0.5 × min(1, 4 / n)`: sparse palettes place detail on their steps, rich ones barely change.
Knots closer than 0.001 are dropped; y never decreases. A curve within 1e-4 of identity becomes the identity curve.

**Shift.** If neutral lies inside the palette hull (reach ≥ 0 every 5°), no shift. Otherwise greys cannot be mixed: shift the opponent axes half-way toward the palette centroid, at most 0.1 long.

**Sectors.** Six hue sectors, every 60°, 60° wide. For each: coloured mass (alpha weight × window × neutral ramp, after the shift), mean chroma, and reach at its centre.

**Chroma.** The mass-weighted mean of `min(1.25, reach / chroma)` over sectors, clamped to `[0, 1.15]`, and snapped to 1 within 0.02. A grey image keeps 1.

**Groups.** For each sector with at least 2% of the coloured mass:
if reach at its centre is below half its mean chroma after the overall scale, turn toward the nearest direction within 45° that reaches that far, trying +5°, −5°, +10°, … in order.
Its chroma is `min(1.25, reach / chroma) / overall`, clamped to `[0.25, 1.1]`, measured at the turned direction.
Together the two scales boost a hue at most about 1.27×: enough to keep muted colours from collapsing onto grey entries, without repainting the image.
Groups with no turn and a chroma within 0.02 of 1 are omitted.

## Why this works this way

Pulling each pixel to its nearest palette entry is quantization, and it throws away the mixtures dithering can show.
The hull is the right model of what a palette can represent through dithering. It is also cheap: one dot product per palette colour.
Compressing only out-of-range tones leaves images alone when a rich palette already fits them; the evaluation showed a full stretch moved every image for no gain.
Sparse palettes gain from placing detail on the steps they have.
Muting the hues the palette cannot mix avoids noisy dithering there. Turning them toward reachable hues keeps them distinct instead of letting them collapse to grey.
Every choice is bounded and explicit, so the recipe stays predictable and easy to edit.

## Correctness invariants

- Deterministic: fixed sampling, sorting with `total_cmp`, fixed iteration order, no randomness.
- The recipe always passes `RecolourRecipe::validate`.
- Changing the image, the palette, or the space changes the analysis input; nothing is cached here.

## Edge cases

- Transparent-only images and palettes without visible colours give the identity recipe.
- A grey image has no coloured mass: chroma stays 1 and no groups are made, in every space.
- A single-lightness palette keeps the identity tone curve.
- A grey palette has zero reach in every hue, so chroma becomes 0: the image turns grey before quantization.

## Production obligations

Production may cache a recipe by the exact image reaching the step, the palette, and the space, but must derive the same recipe.

## Non-goals

Regional or semantic analysis, and iterative optimization. Callers who want more can edit the recipe.
