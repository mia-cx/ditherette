# Continuous colour spaces

## Purpose

Effects that work in linear light or Oklab convert the carrier here. The v1 colour spec converts bytes and clips; these conversions keep every value.

## Inputs and outputs

Triples of `f32`. Carrier RGB is encoded sRGB units; linear RGB is linear light; Oklab is `[L, a, b]`.

## Algorithm / semantic rule

`to_linear` and `from_linear` apply `spec::color`'s piecewise sRGB transfer functions channel by channel.
Values below the linear-segment threshold, including negatives, use the linear segment. Values above 1 use the power segment.

`linear_to_oklab` and `oklab_to_linear` repeat the matrices from [oklab.md](../color/oklab.md), with signed cube roots and cubes.
Nothing is clipped at any step.

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
