# Continuous colour spaces

## Purpose

Effects that work in linear light, Oklab, or the quantization working space convert the carrier here. The v1 colour spec converts bytes and clips; these conversions keep every value.

## Inputs and outputs

Triples of `f32`. Carrier RGB is encoded sRGB units; linear RGB is linear light; Oklab is `[L, a, b]`.

## Algorithm / semantic rule

`to_linear` and `from_linear` apply `spec::color`'s piecewise sRGB transfer functions channel by channel.
Values below the linear-segment threshold, including negatives, use the linear segment. Values above 1 use the power segment.

`linear_to_oklab` and `oklab_to_linear` repeat the matrices from [oklab.md](../color/oklab.md), with signed cube roots and cubes.
`linear_to_cielab` and `cielab_to_linear` repeat [cielab.md](../color/cielab.md): the D65 matrix, `f`, and its inverse.

`to_opponent` gives lightness plus two opponent axes in the selected working space, with neutral colours at zero:

| Space | Coordinates |
| --- | --- |
| `srgb`, `ycbcr` | BT.601 luma and centred Cb/Cr on encoded RGB: `Y`, `(B - Y) / 1.772`, `(R - Y) / 1.402` |
| `linear-rgb` | BT.709 luma and chroma on linear light: `Y`, `(B - Y) / 1.8556`, `(R - Y) / 1.5748` |
| `oklab`, `oklch` | Oklab `L, a, b` |
| `cielab`, `cielch` | CIELAB `L*, a*, b*`, each divided by 100 |

Cylindrical spaces use their cartesian form: turning a hue is a rotation of the opponent plane either way.
`from_opponent` applies the inverse formulas. Nothing is clipped at any step.

## Why this works this way

Clipping between effects would throw away shades a later effect could bring back into range.
The constants are copied rather than imported because the v1 module exposes only byte conversions.

## Correctness invariants

- Input inside the carrier bound (`±64`) gives finite output.
- The round trips are not exact in `f32`. Effects with neutral arguments skip the conversion rather than rely on it.

## Edge cases

Out-of-gamut Oklab values produce linear RGB outside `[0,1]`. The chain boundary clips them per channel.

## Production obligations

Production may tabulate `to_linear` for byte inputs. Otherwise it evaluates these formulas in the same order.

## Non-goals

Gamut mapping. Clipping stays at the RGBA8 boundary.
