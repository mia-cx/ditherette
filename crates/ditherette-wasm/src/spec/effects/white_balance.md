# White balance

## Purpose

Warm or cool an image and correct green or magenta casts: Lumetri's temperature and tint.

## Inputs and outputs

```json
{ "effect": "white-balance", "enabled": true, "temperature": 0.3, "tint": -0.1 }
```

Both arguments are finite and in `[-1, 1]`. Neutral is `0` and `0`.

## Algorithm / semantic rule

Gains, in `f32`: red `2^(0.5 * temperature)`, green `2^(-0.5 * tint)`, blue `2^(-0.5 * temperature)`.
Each channel decodes to linear light, multiplies by its gain, and encodes again. Nothing is clipped.
A channel whose gain is exactly 1 is returned unchanged.

## Why this works this way

A colour cast is a per-channel scale of light, so gains belong in linear light, like a camera's white balance.
Opposing red and blue gains move along the warm–cool axis without changing overall brightness much.
Half a stop per channel at the ends is a strong correction for photos and still controllable for palette work.

## Correctness invariants

- Neutral is an exact no-op; `tint == 0` leaves green untouched even when temperature changes.
- Black stays black.

## Edge cases

Arguments outside `[-1, 1]` fail at `effects.i.temperature` or `effects.i.tint`.

## Production obligations

Per-channel with a different gain per channel. Production may tabulate each channel separately.

## Non-goals

Kelvin units and chromatic adaptation transforms. The sliders are relative, like Lumetri's.
