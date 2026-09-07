# Production resize rules

The S19 canonical `scalar/nearest.rs` starts from a verified literal reference copy.
It now uses the measured exact incremental mapper and retains generic formats and strided views.
The rules below describe inherited kernels, including the unpromoted `scalar/nearest_candidate/`.
See [the baseline record](../../../../../.plans/60-literal-nearest-baseline.md)
and [promotion evidence](../../../../../.plans/60-nearest-measurement.md).

This directory contains optimized internal resize kernels. These rules are kept
here so they are visible while grepping or editing prod resize code. See
[`PERFORMANCE.md`](./PERFORMANCE.md) for the benchmark-driven optimization
playbook.

## Packed RGBA8 is the prod boundary

Production resize kernels assume normalized packed RGBA8:

```text
R G B A  R G B A  R G B A ...
```

No row padding, no subimage stride, no generic pixel formats. HDR, non-RGBA,
padded-row, or subimage inputs must be converted before calling prod resize.

## Do not reintroduce strided prod kernels

Strided image views are useful in the image model and spec tests, but prod resize
should not grow strided fast paths or fallbacks. If a caller has strided data,
pack it at the boundary.

## Runtime checks are developer tripwires

Prod resize is internal to Ditherette, not a public SDK. Prod entrypoints may use
`debug_assert!` to catch architecture violations during development, but should
not add release-mode validation to hot paths unless a real external boundary is
being crossed.

## Nearest word-copy is not a format

Nearest may copy one RGBA8 pixel as an unaligned `u32` internally. That is a
nearest-specific optimization, not a public native-endian pixel layout. Inputs
and outputs remain packed RGBA8 bytes.

## Spec independence

Production code must not import `crate::spec`. Duplicate small formulas when
needed so the executable spec remains an independent oracle.
