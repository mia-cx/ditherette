# S19 nearest promotion

Superseded by [the landed-kernel restoration](108-restore-landed.md). The incremental replacement is removed from current production.
This document records the old experiment, not an instruction to repeat or promote it.

The experiment's accepted baseline is the literal-copy checkpoint `0ede7f6c6f90d6c5d40b169b1dd835f0ac752902`.
This worktree starts from its validated policy join `89b570e0dbb4280352b157bfde5b20c3a7e80a9e`.
The nearest subagent owns the candidate kernel, conformance tests, and implementation evidence in this file.
The coordinator owns benchmark registration and experiment generation in the same worktree.
Other agents own the private Rust processor and public TypeScript wrapper in separate worktrees.

## Experiment

Nearest copies one pixel per output pixel. The direct reference recomputes a u128 quotient for every coordinate.
Test an allocation-free quotient/remainder recurrence with identical integer rounding for all nine anchors.
An unchanged width permits a logical-row copy, including padded storage and vertical-only resizing.
Keep the literal accepted kernel active until measured evidence supports promotion.

Require exact output across supported formats, independent strides, and all anchors.
Check recurrence arithmetic against the frozen mapper at tiny dimensions and near u32 limits.
No reference or shared-image edits are permitted.

Fresh native pairs compare the copied baseline and this candidate with the frozen oracle.
Use identity, reduction, enlargement, unequal-axis, and small-image cases, each with separate latency and throughput evidence.
Target at least 20% lower median time for nontrivial image cases because the candidate removes per-pixel division.
Require no confirmed per-case slowdown above 10%; fresh measurements supply the actual time values.
Budget four alternating pairs per case and one candidate revision, with one repeat only for inconclusive evidence.
Full public browser-call measurements follow in S20 and remain required even if native results pass.

## TODOs

- [x] Implement the separate exact candidate and focused conformance checks.
- [x] Register the candidate, prepare clean binaries, and record the typed experiment before measurement.
- [x] Drain agents and builds, run one exclusive paired trial, and record the measured outcome without automatic non-exact acceptance.
- [x] Mechanically promote the measured kernel and validate canonical production without changing its arithmetic.

No benchmark runs during implementation. All PRs remain unmerged.

## Candidate implementation before measurement

`prod/resize/scalar/nearest_incremental.rs` derives from the verified copied nearest loop at `0ede7f6c6f90d6c5d40b169b1dd835f0ac752902`.
Its public `resize_nearest_into<F: ImageFormat>` retains the accepted function's arguments and return type.
Two private axis states hold quotient/remainder and quotient/remainder increments in u64.
Initialization divides once per component; the pixel loop adds and carries without division or allocation.
All u32 dimensions fit because denominator and step are at most twice u32::MAX, and summed remainders need at most 34 bits.
An unchanged width copies logical rows directly. Neither path touches row padding or performs arithmetic on pixel storage.

Coordinate tests compare the mechanically verified copied mapper, keeping production independent of spec even under cfg(test).
They exhaust dimensions 1 through 65 and seek rational numerators near u32 boundary offsets before checking short recurrence sequences.
Integration fixtures compare the frozen kernel directly across all nine anchors, 49 shape pairs, four stride combinations, and three formats.
The canonical accepted kernel, common alignment, frozen image types, and literal-copy manifest remain unchanged.

Validation before measurement:

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --lib nearest_incremental` passes both coordinate fixtures.
- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --test prod_nearest_incremental --test prod_nearest_baseline` passes eight tests, including the candidate registry invocation and unchanged literal-copy assertions.
- The three candidate image fixtures check 5,292 exact comparisons, including identity and vertical-only row copies for float payloads and palette indices.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects` passes.
- Rust formatting and `git diff --check` pass.
- The separate `v1-s18-freeze` trusted checker passes against this worktree with `--root /home/mia/mia-cx/ditherette/.worktrees/v1-s19-nearest-opt --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze`.
  Its full result remains revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, artifact `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

The initial generic test helper needed a `Copy` bound on its format marker to reuse the same validated view.
After that test-only correction all pre-measurement checks pass.

## Mechanical promotion

The coordinator's authorized native experiment compares accepted `89b570e0` with candidate `f9b51e45`.
All ten cases pass with exact output, 8,000 samples, and 80 reaped children; maximum live benchmark children is one.
The coordinator owns the detailed measurement report in `.plans/60-nearest-measurement.md` and the `target/s19-nearest-trial-01` artifacts.

The measured implementation now lives in canonical `prod/resize/scalar/nearest.rs`; the redundant incremental module is removed.
Only its module documentation changes. A direct comparison confirms every byte after those four documentation lines matches `f9b51e45cad691a207e8a0e346bdd419af7dd909`.
The historical `candidate:resize:nearest:incremental` ID remains an explicit alias of promoted production with the canonical source path.
The legacy nearest candidate stays unchanged. All existing output conformance remains, and the incremental fixtures target canonical production.
Only the historical source-text equality test is removed. The verified `0ede7f6c` commit and its copy manifest remain the provenance record.

This promotion claims native kernel evidence only. Public runtime and browser evidence remains pending S19 integration and S20.
No additional measurements run during promotion.

Promotion validation:

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects` passes all 285 native tests.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects` passes.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --all --check` and `git diff --check` pass.
- The separate trusted guard command recorded above passes again, including independent native/Wasm compilation and the unchanged frozen identity.
- The frozen spec/image closure, legacy candidate, common alignment, and literal-copy evidence have no changes.
