# Exposure

## Purpose

Brighten or darken as if the photo had been exposed longer or shorter, the first control in Lumetri-style basic correction.

## Inputs and outputs

```json
{ "effect": "exposure", "enabled": true, "stops": 0.5 }
```

`stops` is finite and in `[-4, 4]`. Neutral is `0`.

## Algorithm / semantic rule

For each channel: decode to linear light with the extended sRGB curve ([space.md](space.md)), multiply by `2^stops`, and encode again. Nothing is clipped.
`stops == 0` returns the value unchanged.

## Why this works this way

A stop doubles or halves light, so the scaling belongs in linear light. Scaling encoded values would shift hues and crush shadows unevenly.
The neutral shortcut matters because the decode/encode round trip is not exact in `f32`.

## Correctness invariants

- Neutral is an exact no-op.
- Black stays black; ratios between channels in linear light are preserved, so hue does not drift.

## Edge cases

Highlights pushed past 1 stay above 1 until the boundary clips them, or a later effect pulls them back.

## Production obligations

Per-channel, so production may tabulate it.

## Non-goals

Highlight roll-off. Pair exposure with curves for a soft shoulder.
