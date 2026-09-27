# Brightness and contrast

## Purpose

The two most common grading sliders, applied equally to R, G, and B.

## Inputs and outputs

```json
{ "effect": "brightness-contrast", "enabled": true, "brightness": 0.05, "contrast": 0.3 }
```

Both arguments are finite and in `[-1, 1]`. Neutral is `0` and `0`.

## Algorithm / semantic rule

For each channel `v` in encoded sRGB units, in `f32`: `v' = (v - 0.5) * 4^contrast + 0.5 + brightness`.
Neutral arguments return `v` unchanged. The result is not clipped.

## Why this works this way

`4^contrast` makes the slider symmetric in stops: `+0.5` and `-0.5` are inverse slopes (2× and ½×). The ends are 4× and ¼×.
Mid-grey is the conventional pivot. Brightness is a plain offset, the classic "legacy" brightness.
Leaving the result unclipped lets a later levels or curves step recover shades pushed past white or black.

## Correctness invariants

- Neutral arguments are an exact no-op, including for values outside `[0,1]`.
- R, G, and B use the same map, so neutral colours stay neutral.

## Edge cases

Arguments outside `[-1, 1]` fail at `effects.i.brightness` or `effects.i.contrast`.

## Production obligations

Per-channel, so production may tabulate it.

## Non-goals

Highlight or shadow recovery, and contrast in linear light.
