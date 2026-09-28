# Colour models

## Purpose

Define one conversion and normalization layer for effects that address named colour-model channels.

## Inputs and outputs

The input and output are the encoded sRGB carrier. Intermediate coordinates use this canonical order:

| Model | Channels | Normalized coordinates |
| --- | --- | --- |
| `srgb` | red, green, blue | encoded values unchanged |
| `linear-rgb` | red, green, blue | linear values unchanged |
| `hsl` | hue, saturation, lightness | `H / 360`, `S`, `L` |
| `hsv` | hue, saturation, value | `H / 360`, `S`, `V` |
| `oklab` | lightness, a, b | `L`, `a / 0.8 + 0.5`, `b / 0.8 + 0.5` |
| `oklch` | lightness, chroma, hue | `L`, `C / 0.4`, `H / 360` |
| `cielab` | lightness, a, b | `L* / 100`, `a* / 250 + 0.5`, `b* / 250 + 0.5` |
| `cielch` | lightness, chroma, hue | `L* / 100`, `C* / 150`, `H / 360` |
| `ycbcr` | luma, cb, cr | `Y`, `Cb + 0.5`, `Cr + 0.5` |

## Algorithm / semantic rule

All operations use `f32` in written order. HSL and HSV use the standard six-sector formulas on carrier RGB clamped to `[0,1]`. Oklab and CIELAB use the extended conversions in [space.rs](space.md). Their cylindrical forms use `hypot` and `atan2`. YCbCr uses BT.601 on encoded sRGB.

Hue confidence is `clamp(chroma / neutral_chroma, 0, 1)`. HSL and HSV use encoded RGB chroma `0.02`. Oklch uses `0.02`. CIELCh uses `2.0`.

## Why this works this way

The normalized coordinates give every curve the same `0..1` control domain. The opponent-axis ranges cover common editing values and put neutral at `0.5`. HSL and HSV clamp their input because their conventional definitions assume nominal RGB.

## Correctness invariants

- Hue is normalized to `[0,1)` and wraps on reconstruction.
- Exact greys have zero hue confidence.
- Cartesian models, lightness, and chroma are not clamped during conversion.
- All inverse conversions return an unclipped carrier.

## Edge cases

HSL or HSV consumes carrier overshoot when a non-neutral effect converts it. Other models keep using extended conversion formulas. An effect's exact-neutral shortcut avoids every conversion.

## Production obligations

Production must keep the same `f32` constants, operation order, channel order, and hue thresholds.

## Non-goals

Gamut mapping, white-point adaptation, and perceptual rescaling of normalized coordinates.
