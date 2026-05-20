# tiling contract spec

## Purpose and scope

`spec/tiling` defines the generic tiling contract, not operation-specific tiled implementations.

It exists so future production schedulers and benchmark harnesses can agree on what a valid tiling plan means before any domain adapter optimizes work distribution.

It owns:

- output-domain row-band terminology
- complete/non-overlapping coverage rules
- validation of generic row-band plans

It does not own:

- resize/color/dither/quantize adapters
- worker scheduling
- Rayon/thread pools
- source extents/halo computation for a specific operation
- operation-specific cost models

## Module map

```text
contract.rs  RowBand and RowBandPlan contract types
```

## Domain model

Tiling partitions output writes only.

A row band is a half-open output row range:

```text
y_start..y_end
```

A row-band plan is valid when its bands cover every output row exactly once, in order, without gaps or overlaps.

## Why this works this way

Keeping the generic contract tiny prevents tiling from becoming coupled to image formats or processing algorithms. Resize, color conversion, ordered dithering, palette histograms, and other domains can all describe their output work in row bands while keeping domain-specific read dependencies in their own production adapters.

## Correctness invariants

- Row bands are non-empty.
- Row bands are half-open intervals.
- A valid plan covers output rows `0..height` exactly once.
- A valid plan has no gaps.
- A valid plan has no overlaps.
- Tiling says nothing about input cropping.
- Domain adapters process bands using global coordinates.

## Production obligations

Production schedulers may choose band sizes using cost models, worker counts, or benchmark-derived formulas. Any produced plan must still satisfy this contract.

Domain adapters under `prod/<domain>/tiling` must interpret row bands as write ownership only. They may read the full input image or any source extent needed for the output band.

## Non-goals / deferred choices

- No 2D tile contract yet.
- No source halo/extents in the generic contract yet.
- No scheduling algorithm in spec.
- No operation adapters in spec.
