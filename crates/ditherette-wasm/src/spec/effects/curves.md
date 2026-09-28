# Curves

## Purpose

Remap or adjust named colour-model channels with up to 16 ordered curves.

## Inputs and outputs

```json
{
  "effect": "curves",
  "enabled": true,
  "curves": [
    {
      "kind": "remap",
      "x": { "model": "oklch", "channel": "lightness" },
      "y": { "model": "oklch", "channel": "lightness" },
      "points": [[0, 0], [0.5, 0.6], [1, 1]]
    },
    {
      "kind": "adjust",
      "x": { "model": "hsl", "channel": "hue" },
      "y": { "model": "cielch", "channel": "chroma" },
      "points": [[0, 0.5], [0.5, 0.8], [1, 0.5]]
    }
  ]
}
```

Each curve has exactly `kind`, `x`, `y`, and `points`. `x` and `y` select a channel from [model.rs](model.md). The channel name must belong to its model. A `remap` requires the same model and channel on both sides. An `adjust` accepts any valid pair.

The effect accepts zero to 16 curves. Each `points` list contains 2 to 16 `[x, y]` pairs. Coordinates must be finite and in `[0,1]`. Each x must be at least `0.001` above the previous x.

## Algorithm / semantic rule

`Spline` is a monotone cubic Hermite spline with Fritsch–Butland tangents, all in `f32`. With secants `d[k] = (y[k+1] - y[k]) / (x[k+1] - x[k])`:

- End tangents are the adjacent secant.
- An interior tangent is zero when its two secants differ in sign or either is zero.
- Otherwise it is `(w1 + w2) / (w1 / d[k-1] + w2 / d[k])`, where `w1 = 2 h[k] + h[k-1]` and `w2 = h[k] + 2 h[k-1]`.

Evaluation clamps the input to the first and last x. At the last x it returns the last y. Other inputs use the first segment whose right end exceeds the input. For `s = x - x[k]`, `c2 = (3d - 2m0 - m1) / h`, and `c3 = (m0 + m1 - 2d) / h²`, the result is `y[k] + s(m0 + s(c2 + s c3))`.

For each pixel, retain `source` and initialise `current = source`. Apply curves in list order:

1. Convert `source` to the X model and read its X channel.
2. Evaluate that curve's spline at the X value.
3. Convert `current` to the Y model.
4. Edit only the Y channel, convert back to the carrier, and store that result as `current`.

Every curve reads X from `source`. Earlier curves only affect the accumulated Y-side result. Each curve performs its own conversions, including adjacent curves that use the same model.

For a non-hue remap, set Y to the spline result. For a hue remap, find the shortest circular delta from the original hue to the spline target. Multiply it by the original hue confidence, add it to the original hue, and wrap modulo one. Zero confidence leaves `current` unchanged without a Y conversion.

For an adjustment with curve result `c` and confidence `w`:

- hue adds `w * (c - 0.5)` turns;
- saturation or chroma multiplies by `1 + w * (2c - 1)`;
- every other channel adds `w * (c - 0.5)` normalised units.

If X is hue, its confidence comes from `source`. If Y is hue, its confidence comes from `current`. When both are hue, use the smaller confidence. Zero confidence leaves `current` unchanged. Hue output wraps modulo one. Other outputs are not clamped before conversion. A curve whose every y is exactly `0.5` is a neutral adjustment and skips all conversion.

Every remap uses the ordinary open Fritsch–Butland spline, including a hue remap. The exact identity remap `[[0, 0], [1, 1]]` leaves an unchanged carrier untouched. This preserves the old model-curve behavior.

Only an adjustment with a hue X axis uses the cyclic spline. Its first point must have `x = 0`, its last point must have `x = 1`, and those two y values must match exactly. The duplicate last point closes the seam. The seam tangent uses the final and first secants, and evaluation wraps X modulo one.

## Why this works this way

Remaps replace a channel value. Adjustments use one shared neutral midpoint for turns, gains, and offsets. Reading every X from the original input makes selection stable while list order controls accumulated edits.

Fritsch–Butland tangents avoid the ringing of natural cubic splines. The cyclic form avoids a visible corner where hue zero meets hue one.

## Correctness invariants

- An empty curve list preserves the carrier exactly.
- Every non-hue control point maps exactly to its y value.
- Non-periodic spline output stays within the range of its point y values, up to `f32` rounding.
- Exact greys ignore remaps or adjustments that depend on a hue coordinate.
- A colour at half of a hue-confidence threshold receives half the full hue-dependent edit.
- Alpha is unchanged.

## Edge cases

- More than 16 curves fails at `effects.i.curves`.
- An invalid model-channel pair fails at `effects.i.curves.j.x.channel` or `.y.channel`.
- A remap with unequal sides fails at `effects.i.curves.j.y`.
- An invalid point count fails at `effects.i.curves.j.points`.
- Point bounds and gaps fail at `effects.i.curves.j.points.k.0` or `.1`.
- A broken hue-input adjustment seam fails at the first or last point coordinate that violates the seam.
- HSL and HSV consume carrier overshoot when a curve converts through them.

## Production obligations

Production prepares at most 16 resolved curves and inline splines once per call. It may fold a step into channel tables only when every curve is a remap of the same sRGB or linear-RGB channel. All other valid steps remain pointwise and may use the colour memo. Every path must preserve the reference operation order and output bytes.

## Non-goals

Point handles, automatic gamut mapping, masks, two-input curves, and compiled LUTs.
