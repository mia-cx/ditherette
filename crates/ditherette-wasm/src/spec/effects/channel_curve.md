# Channel curve

## Purpose

Use one named colour channel as the horizontal axis of a curve that adjusts another named channel.

## Inputs and outputs

```json
{
  "effect": "channel-curve",
  "enabled": true,
  "x": { "model": "hsl", "channel": "hue" },
  "y": { "model": "oklch", "channel": "chroma" },
  "points": [[0, 0.5], [0.5, 0.8], [1, 0.5]]
}
```

`x` and `y` each select one channel from the models in [model.rs](model.md). The channel name must belong to its model. `points` follows the count, bounds, and gap rules from [curves](curves.md).

The curve's horizontal position selects input colours. Its vertical value is an adjustment, not an absolute output value:

| Y channel | Flat line at `0.5` | Bump above `0.5` | Dip below `0.5` |
| --- | --- | --- | --- |
| hue | keeps the hue | turns the hue forward, up to 180 degrees | turns the hue backward, down to -180 degrees |
| saturation or chroma | keeps the amount | increases it, up to double | reduces it, down to zero |
| every other channel | keeps the value | adds up to `0.5` normalized units | subtracts up to `0.5` normalized units |

A flat line therefore does nothing. A bump changes colours whose X coordinate reaches that part of the curve. A dip changes the same selected colours in the opposite direction.

## Algorithm / semantic rule

Convert the input pixel to the X model and read the selected X coordinate. Convert the same input pixel to the Y model and retain all three original Y coordinates. Evaluate the spline at X, adjust the selected Y coordinate once, then convert the modified Y model back to the carrier.

For curve result `c` and hue confidence `w`:

- hue Y adds `w * (c - 0.5)` turns;
- saturation or chroma Y multiplies by `1 + w * (2c - 1)`;
- every other Y adds `w * (c - 0.5)` normalized units.

If X or Y is hue, `w` is the minimum confidence of every hue-valued side. Otherwise `w` is one. Hue output wraps modulo one. Other Y outputs are not clamped before conversion.

A hue X axis uses a cyclic Fritsch-Butland spline. Its first point must have `x = 0`, its last point must have `x = 1`, and the two y values must match exactly. The duplicate last point closes the seam. The shared seam tangent uses the final and first secants, and evaluation wraps X modulo one.

## Why this works this way

One relationship per effect keeps ordering explicit. Neutral `0.5` gives hue shifts, gains, and offsets one shared control convention. The cyclic spline prevents a visible corner where hue zero meets hue one.

## Correctness invariants

- A curve whose every y value is exactly `0.5` skips all conversion.
- X and the original Y coordinates come from the same input pixel.
- Exact greys ignore every adjustment that depends on a hue coordinate.
- A colour at half of a hue-confidence threshold receives exactly half the full adjustment.
- Alpha is unchanged.

## Edge cases

HSL and HSV consume carrier overshoot when a non-neutral curve converts through them. Periodic hue curves hit every interior knot exactly and treat X values zero and one as the same point. Offsets and gains may push Y outside its nominal model range.

## Production obligations

Every valid instance is pointwise. An instance is per-channel only when X and Y select the same primary component and both models are `srgb` or `linear-rgb`. Production builds one inline spline per call and must preserve the reference operation order.

## Non-goals

Multi-output curves, automatic gamut mapping, and interpolation between different source pixels.
