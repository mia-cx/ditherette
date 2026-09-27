# effects spec

## Purpose and scope

`spec/effects` defines ordered colour effects that run before palette output.
A caller supplies an array of effect instances. Each enabled instance transforms the result of the one before it.
The chain either returns full-colour RGBA8 or feeds the existing terminal resize and dither/quantize stage.

This domain owns the continuous working image, the effect extension contract, the ordered executor, recipe decoding and validation, and every built-in effect's colour math.
It does not own resizing, palette normalization, matching, dithering, or encoding. Those remain the v1 reference.

## Inputs and outputs

```text
source RGBA8 + ordered effects + context
  -> full-colour RGBA8                         apply_effects

source RGBA8 + recipe v2 (effects + terminal settings) + palette
  -> indexed image                             process
```

The context supplies shared inputs that some effects need: the palette and the quantization working space.
Ordinary effects need neither. `process` derives both from its own palette and `match` setting, so a composed chain cannot disagree with final quantization.

## Module map

| File | Role |
| --- | --- |
| [image.rs](image.md) | Continuous working image and its single RGBA8 boundary |
| [chain.rs](chain.md) | `Effect` extension trait, context, and the ordered executor |
| [channel.rs](channel.md) | Channel selection shared by per-channel effects |
| [recipe.rs](recipe.md) | Serializable built-in registry, decoding, and validation |
| [operation.rs](operation.md) | Standalone `apply_effects` and recipe-v2 `process` |
| [space.rs](space.md) | Unclipped linear-light and Oklab conversions |
| [levels.rs](levels.md) | Levels |
| [curves.rs](curves.md) | Monotone tone curves |
| [brightness_contrast.rs](brightness_contrast.md) | Brightness and contrast |
| [exposure.rs](exposure.md) | Exposure in stops |
| [white_balance.rs](white_balance.md) | Temperature and tint |
| [hue_saturation.rs](hue_saturation.md) | Oklab hue, saturation, and lightness |

## Domain model

The working image is straight, unpremultiplied RGB in encoded sRGB units, one `f32` triple per pixel, plus the source alpha bytes.
Byte `k` enters as `k / 255`. Values are not clipped between effects, so later effects see shades that an earlier effect pushed out of range.
Each effect documents its own working space, argument domains, and how it treats values outside `[0,1]`.
Clipping and rounding happen once, when the chain crosses back to RGBA8.

Alpha is never an effect input or output. Every effect reads and writes RGB only, including hidden RGB under zero alpha.
The boundary copies each source alpha byte unchanged.

## Ordering and repetition

The executor applies enabled steps in array order. It never sorts, groups, merges, or skips duplicates.
Two instances of one effect keep independent arguments.
A disabled step keeps its arguments in the recipe and is validated, but it does no pixel work and needs no context.
An empty chain, or one with every step disabled, returns the source bytes.

## Plugin boundary

Built-in effects are compiled into the crate and decoded from a tagged JSON recipe.
Rust consumers can add effects by implementing `chain::Effect` and running `chain::apply_chain` over their own step type.
Adding a built-in means one new module, one registry variant, and its validation. The executor does not change.
JavaScript callbacks, runtime-loaded modules, and executable recipe values are not supported.

## Placement

Effects run first, on the source at its own resolution: `source -> effects -> RGBA8 -> resize -> dither/quantize`.
The RGBA8 boundary before resize is the same boundary any caller-supplied source already has.
Dithering is part of the terminal stage, not an effect. No recipe shape places a colour effect after quantization.

## Correctness philosophy

Every effect is a readable, deterministic `f32` formula with explicit operation order.
Production may fold, fuse, tabulate, or parallelize effects, but exact modes must reproduce these bytes.

## Edge cases and invariants

- Unknown effect names and unknown fields fail with the step's index, never silently skip.
- Arguments are finite and inside each effect's documented domain.
- Effects map finite coordinates to finite coordinates. The executor bounds the carrier to `±64` after each step, so a long chain cannot overflow.
- Validation completes before any pixel work or output allocation.
- A chain holds at most 64 steps, so decoding and validation stay bounded.
- Effects see the palette quantization keeps: the first 256 entries.

## Production obligations

`prod/effects` is an independent copy. It must match `apply_effects` and `process` byte-for-byte for every recipe.
It must not import this module.

## Non-goals / deferred choices

- Alpha-changing effects.
- Spatial effects such as blur or sharpen. The whole-image `apply` signature allows them later.
- Effects after resize.
