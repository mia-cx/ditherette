# Curves

## Purpose

Reshape tone per channel with a smooth curve through a few control points, like the Curves panel in an image editor.

## Inputs and outputs

```json
{ "effect": "curves", "enabled": true, "channel": "rgb", "points": [[0, 0], [0.25, 0.2], [0.75, 0.85], [1, 1]] }
```

`points` holds 2 to 16 `[x, y]` pairs in encoded sRGB units, x strictly increasing. `channel` is `rgb`, `red`, `green`, or `blue`.

## Algorithm / semantic rule

`Spline` is a monotone cubic Hermite spline with Fritsch–Butland tangents, all in `f32`.
With secants `d[k] = (y[k+1] - y[k]) / (x[k+1] - x[k])`:

- End tangents are the adjacent secant.
- An interior tangent is 0 when its two secants differ in sign or either is 0.
- Otherwise it is the weighted harmonic mean `(w1 + w2) / (w1 / d[k-1] + w2 / d[k])`, with `w1 = 2 h[k] + h[k-1]` and `w2 = h[k] + 2 h[k-1]`.

To evaluate `v`, clamp it to `[x[0], x[last]]`. At `x[last]` the result is `y[last]`. Otherwise take the first segment whose right end is above `v`.
With `s = v - x[k]`, `c2 = (3d - 2m0 - m1) / h` and `c3 = (m0 + m1 - 2d) / h²`, the result is `y[k] + s(m0 + s(c2 + s c3))`.

## Why this works this way

Natural cubic splines ring: a curve through `(0.5, 0.6)` can dip below 0 near black. Fritsch–Butland tangents never overshoot and keep monotone curves monotone.
The nested polynomial form keeps straight segments exactly straight, so `[[0,0],[1,1]]` returns `v` unchanged.
Clamping outside the first and last point makes the curve flat there, matching editor curves.

## Correctness invariants

- `[[0,0],[1,1]]` is an exact identity on `[0,1]`.
- Every control point maps exactly to its y value.
- Output stays within the range of the point y values, up to `f32` rounding.
- Values outside `[x[0], x[last]]`, including overshoot from earlier effects, map to the end y values.

## Edge cases

- Fewer than 2 or more than 16 points fail at `effects.i.points`.
- A non-increasing x fails at `effects.i.points.j.0`; out-of-range coordinates fail at `.0` or `.1`.

## Production obligations

Curves is per-channel, so production may tabulate it. The table must evaluate `Spline::eval` on the same `f32` input.

## Non-goals

Luma curves, hue-versus-saturation curves, and point handles. Those need their own effects.
