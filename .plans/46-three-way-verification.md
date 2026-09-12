# #46 Typed three-way verification

Base: `61224131f9bb2e30c98bde685defb884d2a68369`, branch `impl/v1-s05-base`.
Dependencies: S03 `fa3007ff` (PR #89), S04 `b5ed4d25` (PR #90).
Branch: `impl/v1-s05-verification`. No measurements run in this slice.

## TODOs

- [x] Add generic typed cases/subjects and one serializable output contract with full identities.
- [x] Extend the existing RGBA verifier to three-way metadata, index, and coordinate comparisons with failure artifacts.
- [x] Prevent bounded legacy results from promoting accepted baselines.
- [x] Adapt existing resize subjects and S03 storage, validate deterministic fixtures, and document the S17 handoff.
- [x] File an unmerged non-draft PR against the prerequisite join.

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

Adapter validation: three tests exercise concrete S03 resize requests through
the existing spec/prod registry, preserve strided indexed storage and complete
metadata, and reject a missing production mode without substituting spec.
No semantic kernel files changed. Combined benchmark tests pass, including the
guard fixtures. The benchmark Criterion and Wasm bench-subject builds compile;
all three Rust crates pass formatting. `VERIFICATION.md` records S17's handoff.

Delivered in [PR #96](https://github.com/mia-cx/ditherette/pull/96), non-draft and
unmerged against `impl/v1-s05-base`. Validated implementation head:
`1234908aba98383b16dfb0e85ea1c3b38c5f32a2`. This final evidence update changes no code.

## Restack and review amendments, 2026-09-12

Merge `61d46a08` carries updated prerequisite join `4cf944f8`, whose tree matches
main `6a6b35bd`. Its baseline passes 22 focused Rust tests, three controlled Node
transport fixtures, 16 deterministic Node tests, benchmark/Wasm compilation, and
formatting. Historical entries above retain their original heads.

Review amendments add three corrections. The directory and stale-proof fixes
have failing-before/passing-after regressions; the tiling fix has a numeric
probe and a direct trace of the existing caller. `write_review_artifacts` now creates missing ancestors before
the exclusive final bundle directory: the nested-directory variant of
`failures_keep_raw_outputs_and_all_review_images_without_overwriting` failed
with NotFound before and passes after. `require_accepted_verification` binds the
recorded exact proof to the measured output's SHA-256: the controlled one-pixel
`measured_output_must_match_the_verified_invocation` fixture, where the
candidate changes bytes between the verification and measured invocations, was
promotable before and is rejected after, alongside missing and mismatched digest
cases; this fixture proves stale-report rejection only and is not performance
evidence. The tiling sweep's unique bounded validation now tests
`within_bounds` instead of `passed`: a temporary numeric probe
(`tiling_gate_probe.rs`, removed after execution) showed `within_bounds=true`,
`passed=false`, `max_color_distance=1.0` for bounded output, and exact bounds
rejecting the same bytes, while the old caller took the failure branch.
After the changes, `cargo +1.97.0 test --locked --manifest-path
crates/ditherette-bench/Cargo.toml --bin ditherette-bench --test verification
--test verification_adapters -- --test-threads=1` passes 9 binary tests
(including the new digest-binding test and the updated accepted-baseline
fixture), 8 verification tests, and 3 adapter tests;
`cargo +1.97.0 check --locked --manifest-path
crates/ditherette-bench/Cargo.toml --benches`, rustfmt, and `git diff --check`
are clean. Raw evidence is under `/tmp/ditherette-collapse-88.27wHc7/` with
`pr96-fix-*` prefixes. No review or merge completion is claimed here.
