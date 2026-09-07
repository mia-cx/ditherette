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
