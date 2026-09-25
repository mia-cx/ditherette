# Ordered effect chain

## Purpose

This file is the extension contract. Built-in and caller-defined effects implement `Effect`; `apply_chain` runs any list of them.

## Inputs and outputs

`apply_chain` mutates one `EffectImage` in place. It takes ordered `Step`s and an `EffectContext`.
It returns `Ok(())` or the first validation error.

## Algorithm / semantic rule

1. Validate every step's arguments in array order, enabled or not. The error path is `effects[i]` or deeper.
2. For each enabled step, check the context it `needs`. A palette need requires at least one visible colour; a space need requires a working space.
3. Apply enabled steps in array order. Each step reads the image the previous step wrote.

Disabled steps do no pixel work and need no context. Duplicate steps run once per occurrence with their own arguments.

## Why this works this way

The executor only calls trait methods, so adding an effect never touches sequencing.
Validating everything before any work means a bad step late in the chain cannot leave half-applied output.
Whole-image `apply` lets an effect analyse the image it receives, which palette-aware recolouring needs.

`Step<E>` serializes as the effect's own tagged object plus `enabled`:

```json
{ "effect": "levels", "enabled": true, "channel": "rgb", "input": { "black": 0, "white": 1 }, "gamma": 1, "output": { "black": 0, "white": 1 } }
```

## Correctness invariants

- Order is caller order. The executor never sorts, groups, or merges steps.
- A chain with no enabled step leaves the image unchanged.
- Context is read only by effects that declare a need for it.

## Edge cases

- An invalid disabled step still fails validation, so a stored recipe cannot hide a broken entry.
- A missing palette or space for a disabled step is not an error.

## Production obligations

Production may fuse adjacent steps or tabulate per-channel runs, but the result must equal applying each step in order.

## Non-goals

Runtime registration, JavaScript callbacks, and cross-step caching belong to production and the package, not this contract.
