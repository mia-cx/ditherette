# Continuous effect image

## Purpose

`EffectImage` carries pixels between effects without rounding, so a later effect sees every shade an earlier one produced.

## Inputs and outputs

`from_rgba8` reads a packed or strided RGBA8 view. `to_rgba8` returns owned, packed RGBA8 with the same dimensions.

## Algorithm / semantic rule

Each RGB byte `k` becomes the `f32` value `k as f32 / 255.0`. Alpha bytes are stored separately and never converted.
Pixels are row-major; row padding in the source view is not read.

At the boundary each channel is clipped to `[0,1]`, multiplied by 255 in `f32`, and rounded half away from zero.
The source alpha byte is written unchanged.

## Why this works this way

Encoded sRGB units match what levels and curves conventionally operate on. An untouched pixel round-trips to its exact byte.
Effects that need linear light or a perceptual space convert inside their own `apply`.
Keeping alpha out of the carrier means no effect can change transparency by accident.

## Correctness invariants

- `from_rgba8` followed by `to_rgba8` returns the source bytes for every byte value.
- Values outside `[0,1]` survive between effects and clip only at `to_rgba8`.
- After every step the executor clamps channels to `±64` (`bound`). Real chains stay far inside it.
  Without it, a legal chain of repeated boosts can overflow to infinity and then NaN.
- Hidden RGB under zero alpha is processed like any other RGB.

## Edge cases

- One-pixel images and strided sources behave like packed ones.
- Effects must not produce NaN. The boundary has no NaN rule because validated effects cannot create one.

## Production obligations

Production may use planar or packed `f32` layouts, or skip the carrier entirely when a chain is tabulated.
It must produce the same bytes as clipping and rounding this carrier.

## Non-goals

No premultiplication, no gamut mapping, no dithering at the boundary.
