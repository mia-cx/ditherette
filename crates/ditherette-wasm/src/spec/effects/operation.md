# Effect operations

## Purpose

The two public ways to run a chain: return the full-colour result, or continue into palette output.

## Inputs and outputs

`apply_effects` takes `{ version: 1, source, effects, context }` and returns owned RGBA8 at the source size.

`process` takes `{ source, palette, recipe }` where the recipe is:

```json
{ "version": 2, "effects": [...], "output": {...}, "alpha": {...}, "match": "...", "dither": {...} }
```

`output`, `alpha`, `match`, and `dither` are the v1 terminal settings, unchanged. It returns the v1 indexed result.

## Algorithm / semantic rule

`apply_effects`:

1. Reject any version other than 1 at `version`.
2. Validate the chain and its context needs ([chain.md](chain.md)).
3. Validate the source layout with the v1 limits.
4. Decode to an [`EffectImage`](image.md), apply enabled steps in order, and cross the RGBA8 boundary once.

`process`:

1. Reject any version other than 2 at `recipe.version`.
2. Build the context from the request palette and `match.space()`, then validate the chain against it.
3. Validate the terminal settings exactly as v1 `process` would.
4. Run `apply_effects` on the source, then v1 `process` on its RGBA8 output.

Inside `process`, effect paths gain a `recipe.` prefix. A missing palette colour reports `palette`.

## Why this works this way

Defining `process` as a literal composition keeps it reproducible from the public staged calls.
Deriving the context from the recipe means a palette-aware effect always sees the palette and space quantization will use.
Effects run before resize, on the full source, so resize and dither settings can change without re-running effects.

## Correctness invariants

- An empty or fully disabled chain returns the source bytes from `apply_effects`, and v1 `process` output from `process`.
- `process(v2)` equals `pipeline::process(v1)` applied to `apply_effects` output, including indices, palette, and warnings.
- Final indices reference only the normalized request palette, under every alpha policy and dither family.
- No recipe field can run an effect after quantization.

## Edge cases

- Every validation step runs before pixel work, so an invalid terminal setting fails before effects run.
- `version` in `recipe` must be 2. A v1 recipe goes to the frozen v1 `process`.
- The package reports finer paths for shape errors, such as `effects.0.enabled`, where this reference reports `effects.0` with serde's message.

## Production obligations

Production must equal both compositions byte-for-byte. It may cache the effect result and skip effects when only terminal settings change.

## Non-goals

Resizing before effects, and effects that change image dimensions.
