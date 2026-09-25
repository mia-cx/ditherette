# Hue, saturation, and lightness

## Purpose

Turn hues, boost or mute colour, and lighten or darken, like a Hue/Saturation panel, but perceptually even.

## Inputs and outputs

```json
{ "effect": "hue-saturation", "enabled": true, "hue": 20, "saturation": 0.25, "lightness": 0 }
```

`hue` is degrees in `[-180, 180]`. `saturation` and `lightness` are in `[-1, 1]`. Neutral is all zero.

## Algorithm / semantic rule

For each pixel, in `f32`:

1. Convert carrier RGB to linear light, then Oklab `[L, a, b]` ([space.md](space.md)).
2. With `(sin, cos)` of the hue in radians and `k = 1 + saturation`: `a' = (a cos - b sin) k`, `b' = (a sin + b cos) k`.
3. Blend the whole colour toward Oklab white `[1, 0, 0]` when `lightness >= 0`, keeping `1 - lightness` of `a'` and `b'` and moving `L` by `(1 - L) * lightness`.
   For negative lightness, scale all three coordinates by `1 + lightness`, toward black.
4. Convert back to linear light and encode. Nothing is clipped.

All-zero arguments skip the conversion entirely.

## Why this works this way

Oklab is built so that equal hue turns look equal and turning a hue keeps its lightness. HSL hue shifts in sRGB visibly darken yellows and brighten blues.
Rotating the `a`/`b` axes is the same as adding to Oklch hue, without the angle wrap.
Saturation `-1` removes all colour; `+1` doubles chroma. Lightness mirrors Photoshop's blend toward white or black, so `+1` is white and `-1` is black.

## Correctness invariants

- Neutral arguments are an exact no-op.
- Saturation `-1` gives neutral greys (`a = b = 0`) for every input.

## Edge cases

Large saturation pushes colours outside sRGB. They stay unclipped until the chain boundary.

## Production obligations

This effect mixes channels, so it runs on the continuous carrier. Production must evaluate the same formula in the same order.

## Non-goals

Colour-range targeting (only reds, only skies). Palette-aware recolouring (#201) covers targeted colour groups.
