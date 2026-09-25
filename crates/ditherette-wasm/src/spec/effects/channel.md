# Channel selection

## Purpose

Per-channel effects such as levels apply one scalar map to some RGB channels. `Channel` names which ones.

## Inputs and outputs

`Channel::apply` takes one RGB triple in encoded sRGB units and a scalar map. It rewrites the selected channels in place.

## Algorithm / semantic rule

`rgb` maps all three channels with the same function. `red`, `green`, and `blue` map only that channel.
The JSON tags are `"rgb"`, `"red"`, `"green"`, and `"blue"`.

## Why this works this way

Each channel's output depends only on that channel's input. Production can therefore tabulate a run of per-channel effects per byte value.

## Correctness invariants

Unselected channels keep their exact `f32` value.

## Edge cases

Values outside `[0,1]` reach the map unchanged. Each effect defines how it treats them.

## Production obligations

A tabulated production path must evaluate the same map on the same `f32` input.

## Non-goals

Luma or perceptual-lightness channels. Those are not per-channel maps and would need their own effect.
