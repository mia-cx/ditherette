# Recolour

## Purpose

Prepare an image for a limited palette: fit its tones and colours to what the palette can show, directly or through dithered mixtures, before any quantization.
The result stays continuous full colour. Nothing snaps to palette entries here.

## Inputs and outputs

```json
{ "effect": "recolour", "enabled": true, "strength": 1, "recipe": null }
```

`strength` is in `[0, 1]`. `recipe` is `null` to analyse automatically, or an explicit recipe:

```json
{ "space": "oklab", "tone": [[0, 0.05], [0.5, 0.48], [1, 0.95]], "chroma": 0.9, "shift": [0, 0],
  "groups": [{ "hue": 120, "width": 60, "turn": 10, "chroma": 0.7 }] }
```

| Field | Domain | Meaning |
| --- | --- | --- |
| `space` | working space tag | The space the recipe was analysed in, and must be applied in |
| `tone` | curve points, as for [curves](curves.md) | Lightness curve |
| `chroma` | `[0, 2]` | Scale for both opponent axes |
| `shift` | each `[-0.5, 0.5]` | Offset added to the opponent axes first |
| `groups` | at most 12 | Hue-targeted turns and chroma scales |
| `groups[].hue` | `[0, 360]` | Centre, degrees in the opponent plane |
| `groups[].width` | `[1, 180]` | Half-width in degrees |
| `groups[].turn` | `[-180, 180]` | Hue turn in degrees at full weight |
| `groups[].chroma` | `[0, 2]` | Chroma scale at full weight |

A recipe-less step needs the context palette and space. A step with a recipe needs the space, and it must equal `recipe.space`.

## Algorithm / semantic rule

For each pixel, in `f32`, with the lightness–opponent coordinates of [space.md](space.md):

1. `[L, u, v] = to_opponent(rgb, space)`, then `L' = tone(L)` with the curves spline.
2. `u' = (u + shift[0]) * chroma` and the same for `v`.
3. Groups: with `c = hypot(u', v')`, hue `h = atan2(v', u')` in degrees, and `ramp = min(1, c / 0.02)`:
   each group's weight is `window(h) * ramp`, a raised cosine that is 1 at the centre and 0 at `width` away.
   Sum `weight * turn` into one turn and `weight * (chroma - 1)` into one scale offset, clamp the scale at 0, then rotate and scale `(u', v')` once.
4. `adjusted = from_opponent([L', u'', v''], space)`.
5. Strength 0 returns the input; strength 1 returns `adjusted`; otherwise `rgb + strength * (adjusted - rgb)` per channel.

The identity recipe (tone `[[0,0],[1,1]]`, chroma 1, no shift, no groups) returns the input exactly, without the round trip.
With no recipe, the step first derives one from the image it receives ([recolour_analysis.md](recolour_analysis.md)).

## Why this works this way

Analysis and application share one plain recipe, so a caller can inspect it, edit one number, and reapply without analysing again.
Working in the quantization space means the fit is judged the way matching will judge it.
Groups spaced `width` apart form a partition of unity, so neighbouring hues blend smoothly with no seams; the neutral ramp keeps unstable near-grey hues out of every group.
Strength blends in carrier units, so it is exact at both ends and linear between.

## Correctness invariants

- Strength 0 and the identity recipe are exact no-ops.
- Changing only `strength` never changes the recipe; a recipe-less step at any strength analyses the same image.
- Alpha is untouched; hidden RGB is recoloured like any other RGB.
- Output stays continuous. The effect never reads the palette during application.

## Edge cases

- A recipe analysed in `oklab` applied with a `cielab` context fails at `effects.i.recipe.space`.
- Edited groups may overlap and sum past 1; the chroma scale is clamped at 0 rather than flipping hue.

## Production obligations

Production may cache analyses and tabulate per-channel work, but must reproduce these bytes for any recipe and strength.

## Non-goals

Spatial masks and semantic regions. Groups target hue only.
