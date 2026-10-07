# Palette fit

## Purpose

Fit an image's colours to what a palette can represent, like recolour, but through a curve list the
caller can see and edit: analysis writes ordinary [curves](curves.md), and the step applies them
exactly as a `curves` step does.

## Inputs and outputs

```json
{ "effect": "palette-fit", "enabled": true, "look": "fitted", "space": "oklab", "strength": 1, "curves": null }
```

| Field | Domain | Meaning |
| --- | --- | --- |
| `look` | `"natural"`, `"fitted"` or `"vivid"` | Which preset tunes the analysis |
| `space` | `"oklab"` or `"cielab"` | Working space of the analysis and the analysed curves' models |
| `strength` | `[0, 1]` | Scales every curve's bend, see below |
| `curves` | `null` or 0 to 16 [curves](curves.md) | `null` analyses automatically; a list is applied as is (edited, locked) |

Plus the common step envelope (`enabled`, `mask`), exactly as for every other step.

An explicit `curves` list validates exactly like the `curves` effect's list, under
`effects.i.curves`. Its curves may use any colour model, not only `space`'s pair: the user may edit
freely, and the step needs no context then. `curves: null` needs the context palette. `space` is
the step's own; the context's working space is ignored by palette fit.

The model pair per space: `oklab` uses lab model `oklab` and lch model `oklch`; `cielab` uses
`cielab` and `cielch`. The lightness channel is the lab model's `lightness`, which equals the
opponent L of [space.md](space.md) in both spaces.

## Algorithm / semantic rule

Resolve the curve list: the step's own `curves`, or with `curves: null` the analysis of the image
the step receives (unmasked, the whole image) with its `look` and `space` and the context palette
([palette_fit_analysis.md](palette_fit_analysis.md)).

Then apply the list exactly as the `curves` effect does, with per-pixel strength
`m = strength * mask(pixel)` used as the curves effect's mask strength (`1` without a mask).
Strength 0 is therefore an exact no-op without conversion, strength 1 without a mask equals a
`curves` step with the same list byte for byte, and an empty list is an exact no-op.

## Why this works this way

Analysis produces the same curve type the user can draw by hand, so an automatic fit is
inspectable, editable, and stable: editing a curve stores the list and locks the fit until it is
re-analysed. Reusing the `curves` evaluation keeps one definition of what a curve does; the
strength-as-mask-strength rule scales each curve's bend toward neutral rather than blending in RGB.

## Correctness invariants

- Strength 0, an empty list, and an empty analysis are exact no-ops.
- At strength 1 without a mask, output equals a `curves` step with the same list byte for byte.
- A step with explicit curves never reads the palette or the context space.
- Alpha is untouched; hidden RGB is fitted like any other RGB.

## Edge cases

- A transparent-only image, or a palette without visible colours, analyses to `[]`: a no-op.
- A grey image analyses to tone and shift only: no chroma gain, no hue turn.
- Edited curves with models outside the step's pair are still valid and applied verbatim.

## Production obligations

Production must reproduce the reference byte for byte, for explicit lists and for analysed ones.
It may cache a step's resolved curves by the exact image reaching the step, the palette, the step's
`space` and `look`, and the analysis version; strength and mask are not part of the key.

## Non-goals

Strengths above 1.
