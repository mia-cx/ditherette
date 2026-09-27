# Levels

## Purpose

Remap tonal range per channel: choose which input values become black and white, bend the midtones, and choose the output range.

## Inputs and outputs

```json
{ "effect": "levels", "enabled": true, "channel": "rgb",
  "input": { "black": 0, "white": 1 }, "gamma": 1, "output": { "black": 0, "white": 1 } }
```

All points are encoded sRGB units. `channel` is `rgb`, `red`, `green`, or `blue` ([channel.md](channel.md)).

## Algorithm / semantic rule

For each selected channel value `v`, in `f32`:

1. `t = clamp((v - input.black) / (input.white - input.black), 0, 1)`
2. `s = t` when `gamma == 1`, otherwise `s = t.powf(1 / gamma)`
3. `v' = output.black + (output.white - output.black) * s`

Gamma above 1 brightens midtones, matching the Photoshop convention.

## Why this works this way

Clipping to the input range is what makes levels useful: everything below input black becomes output black.
Gamma 1 skips the power so neutral levels are an exact identity for byte input.
Output black may exceed output white, which inverts the channel.

## Correctness invariants

- Neutral arguments return `v` unchanged for every `v` in `[0,1]`.
- Output always lies between output black and output white.
- Values outside `[0,1]` from earlier effects are clipped by step 1. Levels is not transparent to overshoot.

## Edge cases

- `input.black >= input.white` is rejected at `effects.i.input`.
- Every point must be finite and in `[0,1]`. Gamma must be finite and in `[0.1,10]`.

## Production obligations

Levels is a per-channel map, so production may tabulate it for byte input. The table must evaluate this formula on `k / 255`.

## Non-goals

Auto levels, per-channel arguments in one instance, and luma or perceptual-lightness modes. Use one instance per channel.
