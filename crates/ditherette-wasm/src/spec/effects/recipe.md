# Built-in effect registry

## Purpose

`BuiltinEffect` lists every effect the crate compiles in and dispatches the `Effect` trait to each one.
It is also the serializable recipe form callers store and replay.

## Inputs and outputs

`decode_effects` takes a JSON array and returns ordered `EffectStep`s. Each step is one object:

```json
{ "effect": "<name>", "enabled": true, ...arguments }
```

`effect` selects the variant. `enabled` is required. The remaining fields are that effect's arguments, with no extras allowed.

## Algorithm / semantic rule

The array is parsed first, then each element is decoded on its own.
The first element that fails reports `effects.i` with serde's explanation, such as an unknown effect name or field.
Decoding checks shape only. Argument ranges are checked by `Effect::validate`.

Registered effects:

| `effect` | Module |
| --- | --- |
| `levels` | [levels.md](levels.md) |
| `curves` | [curves.md](curves.md) |
| `brightness-contrast` | [brightness_contrast.md](brightness_contrast.md) |
| `exposure` | [exposure.md](exposure.md) |
| `white-balance` | [white_balance.md](white_balance.md) |
| `hue-saturation` | [hue_saturation.md](hue_saturation.md) |

## Why this works this way

A closed enum makes every built-in visible in one place and keeps decoding strict.
Adding an effect is one module, one variant, and one arm in each dispatch method. Sequencing lives in [chain.md](chain.md) and does not change.

## Correctness invariants

- Unknown effect names and unknown fields are errors, never ignored.
- Serializing a decoded step and decoding it again yields the same step.

## Edge cases

- A non-array `effects` value fails at `effects`.
- Duplicate JSON keys follow serde_json: the last value wins. Callers should not rely on this.

## Production obligations

Production decodes the same JSON shape and must reject exactly what this registry rejects.

## Non-goals

Runtime registration and versioned per-effect schemas. A breaking argument change adds a new effect name.
