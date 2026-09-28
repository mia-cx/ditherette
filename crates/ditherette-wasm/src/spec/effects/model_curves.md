# Model curves

## Purpose

Reshape all three channels of one colour model in one ordered effect.

## Inputs and outputs

```json
{
  "effect": "model-curves",
  "enabled": true,
  "model": "oklch",
  "curves": [
    [[0, 0], [1, 1]],
    [[0, 0], [0.5, 0.65], [1, 1]],
    [[0, 0], [1, 1]]
  ]
}
```

`model` is `linear-rgb`, `hsl`, `hsv`, `oklab`, `oklch`, `cielab`, `cielch`, or `ycbcr`. The exact three-item `curves` tuple follows the channel order in [model.rs](model.md). Each item follows the existing [curves](curves.md) point rules. Encoded sRGB uses `curves`, not `model-curves`.

## Algorithm / semantic rule

Convert carrier RGB to the selected model's normalized coordinates. Evaluate one `Spline` per coordinate. Denormalize the three results and convert back to the carrier.

For a hue coordinate, evaluate the absolute target hue. Take the shortest circular delta from the original hue. Multiply that delta by the model's hue confidence, add it to the original, then wrap modulo one.

Normalized inputs pass directly to `Spline::eval`. The spline clamps to its first and last knot. Denormalization does not clamp its result.

## Why this works this way

One exact tuple makes channel order visible and rejects missing channels. Circular interpolation avoids a long turn across the hue seam. Hue confidence stops arbitrary grey hue coordinates from tinting near-neutral colours.

## Correctness invariants

- Three exact identity curves skip conversion and preserve every carrier value exactly.
- A full-confidence hue maps to the spline's absolute target by the shortest circular route.
- An exact grey keeps its original hue-dependent result.
- The effect preserves alpha.

## Edge cases

Every tuple item must contain 2 to 16 valid points. Errors name `effects.i.curves.j` and the failing point coordinate. Hue output `1` is the same angle as `0`. HSL and HSV clamp carrier input before conversion.

## Production obligations

`linear-rgb` is per-channel and may use the channel table. Every other model uses the pointwise memo. Production builds the three inline splines once per call and must match reference bytes.

## Non-goals

Encoded sRGB curves and curves that use one channel to adjust another channel.
