# tiling contract spec

## Purpose

`contract.rs` defines the smallest executable contract for row-band tiling.

It gives future production schedulers and benchmark harnesses a way to validate that an output-domain plan is structurally correct without importing any image-processing algorithm.

## Inputs and outputs

Input:

```rust
ImageDimensions
Vec<RowBand>
```

Output:

```rust
Option<RowBandPlan>
```

A plan exists only if the bands are a complete, contiguous, non-overlapping partition of output rows.

## Algorithm / semantic rule

A `RowBand` is valid when:

```text
y_start < y_end
```

A `RowBandPlan` is valid when iterating bands in order produces:

```text
0..height
```

with no gaps, overlaps, or rows beyond the output height.

## Why this works this way

The tiling contract should be independent from resize/color/dither details. Row-band validity is purely about output ownership. Domain adapters can later decide which input rows, source extents, scratch buffers, or dependencies are needed to compute those output rows.

## Correctness invariants

- Bands are half-open.
- Bands are non-empty.
- A plan covers all output rows exactly once.
- A plan does not imply input cropping.
- A plan does not imply parallel safety by itself; domain adapters must still prove their writes and dependencies are safe.

## Edge cases

Tests cover:

- empty row bands rejected
- complete contiguous coverage accepted
- gaps rejected
- overlaps rejected
- short coverage rejected

## Production obligations

Production schedulers may optimize how plans are generated, but generated plans must satisfy this contract before a tiled adapter executes them.

## Non-goals

- No operation-specific adapters.
- No 2D tiling.
- No source halos.
- No worker scheduling.
