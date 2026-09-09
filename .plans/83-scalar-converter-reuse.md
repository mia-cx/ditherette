# S41 scalar converter reuse candidates

Base PR #134 at `5da82d122` on `perf/v1-scalar-converter-reuse`.
The coordinator selects the field-only candidate after fresh scalar evidence.
The separate Yliluoma target-reuse candidate remains held and is absent from this branch.

## Scope

Restore the held field candidate from `b237b7468fa5fc349760bc0086bd1748113b6d82` into the current row loops.
Reuse its call-owned converter for source and adaptive-neighbor conversion.
Keep current band, progress, byte-boundary, alpha, and memory accounting behavior.
Move only the existing working-space match into the historical `PackedSpace::from_working` helper.

Restore Yliluoma target reuse from `abe8241fe600dd965d265b15858db09bbfb2e14f` after the field commit.
Use the existing prepared-converter accessor without changing mixture search or placement inputs.
The frozen spec, landed color arithmetic, resize, shared storage, and benchmark adapters remain unchanged.

Historical evidence lives in [67-measurement.md](67-measurement.md) and [70-benchmark-results.md](70-benchmark-results.md).
Those incomplete selection gates do not establish current acceptance.
The coordinator owns benchmarks, browser measurements, stacking, and selection.

## TODOs

- [x] Restore field converter reuse and verify frozen output, bands, and capacity boundaries; commit separately.
- [ ] Restore Yliluoma target converter reuse and verify current scalar, band, progress, and complete-call paths; commit separately.

## Validation

Use this worktree's `target/scalar-converter-reuse` with `CARGO_BUILD_JOBS=2`.
Run focused native suites already covering the changed call paths.
Drain every owned job before handing control to the coordinator.

Field validation passes 16 tests across `prod_fields`, `prod_blue_noise`, `prod_field_row_bands`,
`prod_field_band_allocation`, and `prod_processor_fields`.
The existing vectors cover all working spaces, strength extremes, alpha, padding, field draws, and adaptive edge bits.
Current disjoint bands and exact/one-under capacity checks pass without edits to their tests.
`rustfmt --check` on the three changed modules and `git diff --check` pass.
Placement and packed conversion modules match historical candidate `b237b7468` byte for byte.
