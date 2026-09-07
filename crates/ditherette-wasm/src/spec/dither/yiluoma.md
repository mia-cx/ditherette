# Yliluoma two-color ordered reference

This preserves the existing Rust recipe, not a new generalized Yliluoma algorithm.
For Bayer width n in 2,4,8,16, the search uses `levels=n*n`.
Initialize the winner to the first visible palette entry as a pure color.
Enumerate low offset ascending, high offset from low through the last visible entry, then high count from zero through levels inclusive.
For each candidate, set `high_ratio=high_count/levels` and `low_ratio=1-high_ratio` in f32.
Each mixed coordinate is `low_coordinate*low_ratio+high_coordinate*high_ratio`, evaluated in that order.
Replace the winner only for a strictly smaller selected metric score. Equal scores retain the first candidate.
The labels low/high describe enumeration order, not brightness.

`best_ordered_mix_by_distance` preserves the old coordinate-slice callback API.
`best_matched_mix` uses `PaletteMatcher` and returns the original palette indices, including gaps left by Transparent entries.
Both execute the same exhaustive search, with no nearest-pair shortcut, search pruning, or cached mixtures.

## Hue and neutral coordinates

Every coordinate interpolates componentwise, including cylindrical hue. No shortest-arc interpolation or neutral-hue correction is applied to mixtures.
For equal L/C and hues 0.1 and TAU-0.1, the midpoint is PI, not zero.
The selected Euclidean, circular-chord, or minimum-chroma-arc metric still scores that mixture.
Thus neutral hue is ignored by the cylindrical metrics, but remains a coordinate for Euclidean LCH matching.
This behavior is part of the approved [S16 recipe](../../../../../docs/plans/ditherette-v1/slices.md#s16).

## Typed adaptive composition

`dither_yiluoma(DitherQuantizeRequest)` validates the complete request before preparing the palette or allocating output.
It accepts only the Yliluoma family. Other families return `unsupported-operation` at `dither.family`.
S09's `PreparedPalette` owns truncation, alpha preparation, Transparent exclusion, and warnings.
Pixels assigned a fixed alpha-policy index bypass search, including every pixel in a Transparent-only palette.
Visible pixels use the alpha-prepared byte RGB for conversion into the selected matching space.

Find the nearest visible palette coordinate with S10's selected metric and first-entry ties.
Evaluate S12 placement on the original source RGB, before alpha compositing, in that same space.
Weighted RGB matching uses sRGB coordinates for both target and placement.
Adapt every coordinate with `target=nearest+mask*(source-nearest)` before pair search.
The arithmetic is f32. Mask zero returns the exact nearest coordinate; mask one returns the exact source coordinate.
These endpoint branches avoid roundoff in an algebraic identity. Neither branch bypasses mixture enumeration.
Intermediate hue adaptation is componentwise too, not a shortest-arc blend.

Search the adapted target, then evaluate the global Bayer threshold `(rank+0.5)/(n*n)`.
Choose the high entry only if threshold is strictly less than its ratio; otherwise choose the low entry.
The output keeps original retained palette indices and owns normalized palette metadata and ordered warnings.
No RGB reconstruction or separable perturbation boundary exists in this palette-mixing family.

## Zero placement is target adaptation

A zero mask does not guarantee a flat nearest-index output because earlier exact mixtures keep their enumeration priority.
For palette `[black,gray128,gray64]`, source gray64's nearest index is 2.
Pair `(0,1)` with high ratio 1/2 exactly matches that target and wins before pure entry 2.
With Bayer 2, a constant 2x2 source therefore emits `[1,0,0,1]`, even at zero placement.
This fixture preserves the approved target formula and first-pair ordering together.
A zero-mask nearest-only shortcut would change the recipe.
