# #46 Typed three-way verification

Base: `61224131f9bb2e30c98bde685defb884d2a68369`, branch `impl/v1-s05-base`.
Dependencies: S03 `fa3007ff` (PR #89), S04 `b5ed4d25` (PR #90).
Branch: `impl/v1-s05-verification`. No measurements run in this slice.

## TODOs

- [x] Add generic typed cases/subjects and one serializable output contract with full identities.
- [x] Extend the existing RGBA verifier to three-way metadata, index, and coordinate comparisons with failure artifacts.
- [x] Prevent bounded legacy results from promoting accepted baselines.
- [ ] Adapt existing resize subjects and S03 storage, validate deterministic fixtures, and document the S17 handoff.
- [ ] File an unmerged non-draft PR against the prerequisite join.

## Acceptance

All three roles must match operation, space, dimensions, input/settings identity,
and the named reference semantics. Palette order, transparency, and warnings are
hard requirements. Numeric bounds alone cannot approve a non-exact candidate.
Missing comparisons remain incomplete. Failed results retain raw output and
available rendered reference/accepted/candidate/difference PNGs.

## Decisions and validation

The generic case parameter reuses concrete S03 requests without moving public
contracts into a benchmark dependency. S17 connects completed reference adapters;
pre-freeze fixtures cannot establish frozen-spec release conformance.

The join passed 8 request/storage and 6 lifecycle contract tests, S04 Rust
ownership/parser fixtures, 3 Node cleanup fixtures, and the Criterion compile check.

Typed contract validation: bench-api and existing benchmark executables compile.
Coordinate artifact serialization stores every f32 bit, including invalid values
that a failed conformance record must preserve rather than turn into JSON null.

Verifier validation: eight deterministic integration tests pass. They cover all
operation representations, exact/provisional status, required evidence, hard
metadata failures, indexed RGBA max/mean/RMS, separate coordinate errors,
non-finite raw preservation, SHA-256/canonical settings, and immutable PNG bundles.
Existing benchmark executables compile against the extracted shared RGBA engine.

The shared RGBA report now separates `within_bounds` from exact `passed`.
Legacy resize stores the actual verification report with each measured row.
Every accepted-baseline write or indexed refresh requires complete exact proof;
all cases validate before replacement. A one-pixel fixture confirms that
`--allow-correctness-failures` cannot overwrite an accepted artifact with bounded
non-exact output. Missing verification also fails closed.
