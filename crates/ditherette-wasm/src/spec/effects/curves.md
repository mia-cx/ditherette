# Curves

## Purpose

Remap or adjust named colour-model channels with up to 16 ordered curves. An adjustment can read one input curve or a two-input control grid.

## Inputs and outputs

A one-input curve has exactly `kind`, `x`, `y`, and `points`:

```json
{
  "kind": "adjust",
  "x": { "model": "hsl", "channel": "hue" },
  "y": { "model": "oklch", "channel": "chroma" },
  "points": [[0, 0.5], [0.5, 0.8], [1, 0.5]]
}
```

A two-input curve has exactly `kind`, `x`, `x2`, `y`, and `grid`. Its kind must be `adjust`:

```json
{
  "kind": "adjust",
  "x": { "model": "hsl", "channel": "hue" },
  "x2": { "model": "oklch", "channel": "lightness" },
  "y": { "model": "oklch", "channel": "chroma" },
  "grid": {
    "columns": [0, 0.25, 0.5, 0.75],
    "rows": [0, 0.5, 1],
    "values": [
      [0.5, 0.7, 0.5, 0.3],
      [0.5, 0.9, 0.5, 0.2],
      [0.5, 0.6, 0.5, 0.4]
    ]
  }
}
```

`x`, `x2`, and `y` select channels from [model.rs](model.md). Each channel name must belong to its model. A two-input curve requires `x2 != x`.

The effect accepts zero to 16 curves. A one-input `points` list contains 2 to 16 `[x, y]` pairs. A grid contains 2 to 48 columns and 2 to 16 rows. `values[row][column]` belongs to `rows[row]` on `x2` and `columns[column]` on `x`. Every value is finite and in `[0,1]`. Exactly `0.5` is neutral.

## Spline rule

Every spline is a monotone cubic Hermite spline with Fritsch–Butland tangents. All decoded numbers and arithmetic use `f32`. With secants `d[k] = (v[k+1] - v[k]) / (p[k+1] - p[k])`:

- Open end tangents equal the adjacent secant.
- A knot tangent is zero when its two secants differ in sign or either is zero.
- Otherwise the tangent is `(w1 + w2) / (w1 / d[k-1] + w2 / d[k])`, where `w1 = 2 h[k] + h[k-1]` and `w2 = h[k] + 2 h[k-1]`.

For `s = x - p[k]`, `c2 = (3d - 2m0 - m1) / h`, and `c3 = (m0 + m1 - 2d) / h²`, a segment returns `v[k] + s(m0 + s(c2 + s c3))`.

An open sequence clamps its input to its first and last positions. It returns the last value at the last position. Its positions start at `0`, end at `1`, and increase by at least `0.001`.

A hue axis is a closed sequence of unique positions in `[0,1)`. It has no duplicate seam point. Define its final segment as:

```text
p[n] = p[0] + 1
v[n] = v[0]
```

Each knot tangent uses the wrapped previous and next secants. Evaluation wraps the input with `rem_euclid(1)`. An input below `p[0]` uses the final segment to `p[0] + 1`. The wrapped seam gap is `1 - p[last] + p[0]` and must be at least `0.001`.

Two positions define two closed segments. Their opposing secants give both knots zero tangents under the same rule.

One-input curves retain their existing spline form. Every remap uses an open spline, including a hue remap. A hue-input adjustment retains its duplicate endpoint at `0` and `1`, with equal values at those endpoints.

## Evaluation rule

For each pixel, retain `source` and initialise `current = source`. Apply curves in list order. Every later curve still reads its inputs from `source`, while its output edits `current`.

For a two-input curve:

1. Convert `source` to `x.model` and read `x_value`.
2. Convert `source` to `x2.model` and read `x2_value`. The conversion may be reused when both models match.
3. Evaluate each value row along `columns` at `x_value`, in ascending row order.
4. Evaluate the resulting row values along `rows` at `x2_value`.
5. Clamp the grid result to `[0,1]`.
6. Start the confidence `w` at `1`. If `x` is hue, replace it with the source confidence for `x`.
7. If `x2` is hue, set `w` to the smaller of itself and the source confidence for `x2`.
8. Return `current` without a Y conversion when `w == 0`.
9. Convert `current` to `y.model`.
10. If `y` is hue, set `w` to the smaller of itself and the current confidence for `y`. Return `current` when it becomes zero.
11. Apply the adjustment below, then convert the edited Y coordinates back to the carrier.

For an adjustment result `c` and confidence `w`:

- hue becomes `(y + w * (c - 0.5)).rem_euclid(1)`;
- saturation or chroma becomes `y * (1 + w * (2c - 1))`;
- every other channel becomes `y + w * (c - 0.5)`.

Do not clamp the edited Y coordinate before conversion. A two-input grid whose values are all exactly `0.5` skips every colour conversion.

One-input evaluation remains unchanged. It reads X from `source`, evaluates `points`, converts `current` to Y, and applies its remap or adjustment. A non-hue remap replaces Y. A hue remap follows the shortest circular delta from the original hue and scales that delta by source hue confidence. The exact identity remap `[[0, 0], [1, 1]]` preserves an unchanged carrier without conversion. A one-input adjustment whose values are all exactly `0.5` also skips every conversion.

## Mask strength

A [mask](mask.md) gives each pixel a strength `m` from the step's input. Every curve in the step uses the same `m`, which scales how far the curve bends from its neutral:

- `m = 0` leaves the pixel unchanged, without any conversion.
- `m = 1` runs every curve exactly as above.
- Otherwise, with source input value `x` and spline value `f`, a non-hue remap writes `x + m * (f - x)`. A hue remap scales its shortest circular delta: `delta = m * ((f - x + 0.5).rem_euclid(1) - 0.5)`, before the confidence weighting above.
- An adjustment's value `c`, one-input or clamped grid, becomes `0.5 + m * (c - 0.5)` before the hue, gain, or offset formula.

A step without mask curves always uses `m = 1`.

Only palette fit can take `m` past 1, since its strength runs to 3. The bend then overshoots: the
scaled result is *not* clamped as a curve value, but the final channel clamps after the formula and
before converting back. Hue wraps as it already does; saturation and chroma floor at 0; every other
channel clamps to its normalised `[0, 1]`. For `m <= 1` the bend cannot overshoot, so nothing
clamps and every byte matches a step that never scaled.

## Validation order and paths

Two-input validation checks `kind`, `x`, `x2`, `y`, then `grid`.

- A kind other than `adjust` fails at `effects.i.curves.k.kind`.
- An invalid second channel fails at `effects.i.curves.k.x2.channel`.
- An `x2` equal to `x` fails at `effects.i.curves.k.x2`.
- Missing, extra, or mixed `points` and `grid` fields fail strict curve decoding.
- Invalid column and row counts fail at `.grid.columns` and `.grid.rows`.
- Invalid positions fail at `.grid.columns.j` or `.grid.rows.j`.
- A cyclic seam below `0.001` fails at the axis's first position.
- A `values` row count unequal to `rows.length` fails at `.grid.values`.
- A value row width unequal to `columns.length` fails at `.grid.values.r`.
- A non-finite or out-of-range value fails at `.grid.values.r.c`.

Existing one-input validation order and paths stay unchanged. A remap requires the same `x` and `y`. Point coordinates are finite and in `[0,1]`, with an x gap of at least `0.001`.

## Correctness invariants

- An empty curve list preserves the carrier exactly.
- Every open or closed control knot evaluates to its stored value.
- Grid evaluation always runs along `x` first and `x2` second.
- Both grid inputs come from the curve step's original pixel.
- Earlier curves affect only the accumulated output.
- Hue confidence is the minimum confidence from every hue-valued side among `x`, `x2`, and `y`.
- Exact greys ignore adjustments that depend on a hue coordinate.
- One-input behaviour and validation remain byte-identical.
- Alpha is unchanged.

## Production obligations

Production may fold a step into channel tables only when every curve is a one-input sRGB or linear-RGB remap. Any two-input curve makes the step pointwise and uses the colour memo. Neutral grids must be removed before any model conversion.

Prepared grid metadata stores resolved channels and grid indices rather than copying maximum grids into every prepared entry. Per-pixel grid evaluation uses fixed `[f32; 16]` row scratch and performs no heap allocation. Every path preserves the reference `f32` operation order and output bytes.

## Non-goals

Point handles, automatic gamut mapping, and compiled two-dimensional lookup tables.
