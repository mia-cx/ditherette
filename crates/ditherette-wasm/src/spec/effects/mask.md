# Step masks

## Purpose

Limit any effect step to part of the image, such as "only the shadows" or "only the reds". A mask is curves whose output is the step's strength per pixel.

## Inputs and outputs

A step's optional `mask` sits next to `enabled` and holds zero to 4 curves:

```json
{
  "effect": "exposure",
  "enabled": true,
  "mask": [
    { "x": { "model": "oklch", "channel": "lightness" }, "points": [[0, 1], [0.5, 0.2], [1, 0]] },
    {
      "x": { "model": "oklch", "channel": "hue" },
      "x2": { "model": "oklch", "channel": "chroma" },
      "grid": { "columns": [0, 0.5], "rows": [0, 1], "values": [[1, 0], [1, 0.5]] }
    }
  ],
  "stops": 1
}
```

A one-input mask curve has exactly `x` and `points`. A two-input mask curve has exactly `x`, `x2`, and `grid`. Channels, points, and grids follow [curves](curves.md), including its limits and validation paths. A hue `x` on a one-input mask curve uses the cyclic spline, so its points start at `x = 0`, end at `x = 1`, and repeat the first `y`. Values are strengths: `1` is the full step and `0` leaves the pixel unchanged.

A missing `mask` and an empty one both mean strength 1 everywhere. Serialization leaves an empty mask out.

## Algorithm / semantic rule

Masks read the pixel entering the step, after earlier steps and before this one. For each pixel, in `f32`:

1. Start at `m = 1`. For each mask curve in list order, multiply `m` by its value.
2. A one-input curve converts the pixel to `x.model`, evaluates its spline at the `x` coordinate, and clamps to `[0,1]`.
3. A two-input curve evaluates its grid at `x` and `x2` exactly as a curves grid does, clamped to `[0,1]`.
4. Hue confidence `w` starts at 1. A hue `x` sets it to that pixel's confidence for `x`. A hue `x2` lowers it to the smaller of itself and the confidence for `x2`. Confidence comes from [model.rs](model.md).
5. When `w < 1`, the value `v` becomes `1 - w * (1 - v)`. So exact greys see 1 from every hue-keyed curve.

The effect then uses `m` per pixel:

- **Curves** scales every curve's bend from neutral by `m`, as [curves](curves.md) describes.
- **Recolour** multiplies its `strength` by `m`, as [recolour](recolour.md) describes.
- **Every other effect** runs as usual, then each channel moves back toward the input: `in + m * (out - in)`.

At `m = 1` the pixel gets the unmasked result exactly. At `m = 0` it keeps its input exactly. The executor bounds the carrier after the step, as for any step.

## Why this works this way

Masks only hold back, so a mask can never push a step past its own arguments. Multiplying curves means "shadows" and "reds" together select shadows that are also red. Reading the step's input, not its output, keeps the selection stable while the step's arguments change. A per-pixel value that depends only on that pixel keeps the chain pointwise, so exact per-colour tables still work.

## Correctness invariants

- A step with no mask curves behaves exactly as before masks existed.
- Mask curves whose values are all 1 leave every pixel at the unmasked result.
- Every mask value lies in `[0,1]`, so `m` does too.
- Alpha is unchanged.

## Edge cases

- More than 4 curves fail at `effects.i.mask`.
- Curve errors name `effects.i.mask.k`, then the curve's own path, such as `.points.2.0` or `.grid.values.1.3`.
- An `x2` equal to `x` fails at `effects.i.mask.k.x2`.
- A disabled step's mask is validated too.
- Mask validation runs after the step's own arguments.

## Production obligations

Production must reproduce these bytes. It may skip curves whose values are all 1, evaluate masks inside its colour memo, and keep a masked step out of per-channel tables.

## Non-goals

Spatial masks, feathering, and inverting a mask without editing its curves.
