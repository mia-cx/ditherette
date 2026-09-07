# #58 Complete the five-method reference processor

## Summary

Compose the completed references before S18 freezes them. Keep every PR unmerged.

## Validated prerequisite join

Base branch `impl/v1-s17-base` at `4f4b48a04c0ccee51222b7e621ff4a566a495613` contains S05, S10, S11, S13, S14, S15, and S16.
Its 226 listed native tests, Wasm compilation, and formatting checks pass.
The join connects the corrected S14 blue-noise tile to S13's field dispatch.

## TODOs

- [x] Compose all five typed methods and test both public equalities, metadata, warnings, input preservation, and durable results.
- [x] Complete strict tagged requests and executable lifecycle, cache/memory, and partition reference models before freeze.
- [x] Register callable benchmark subjects for every completed reference and reconcile the exhaustive export inventory.
- [ ] Validate the full join, update the stack ledger, and open an unmerged S17 PR against the verified base.

## Ownership

The coordinator owns `spec/pipeline/`, its tests, `spec/mod.rs`, integration joins, this plan, `slices.md`, and the ledger.
The strict-request agent owns existing enum-use corrections and request validation fixtures in its isolated worktree.
The inventory agent owns executable adapters, partition references, and the remaining thread-pool lifecycle model.
The benchmark agent owns benchmark subject registration and adapter fixtures in its isolated worktree.

## Gates

No production optimization or spec freeze occurs here. Benchmark registration runs tests without measurements.
Only the coordinator grants a quiet measurement phase after all implementation and build/test work exits.
The first S06 control trial rejected its candidate; it does not establish accepted production performance.

## Evidence

The completed join passes 270 native tests from actual result-group counts, with no doctests.
The Wasm target compiles with `bench-subjects`; all three crate formatting checks and diff checks pass.
The benchmark crate passes 5 reference-subject, 8 verification, 3 storage-adapter, and 3 binary tests without measurements.
Its Rust lease fixture also passes, including all three owned Node transport cleanup fixtures and controlled subprocess checks.
Native benchmark binaries and benches compile. An extra direct Node fixture invocation rejects the missing inherited lease as designed.
The intended Rust lease runner supplies that lease and passes the fixtures; no guard was bypassed or changed.

An independent read-only pipeline/control review found no concrete defects at `7da4d40a935026c19a4cd1191dc088a41eb16b61`.
The source subtasks were strict/cache model `1e04974ed56cb4ec401c49211b7311e8839a04b8`,
adapter/pool model `b441892b7c5428d1da7b0ffeddbf78afc8ae78a4`, and registration `993bca98ae97cb175b7e8709a626df4dadeed5d0`.
The required rebase onto the unchanged immediate base replayed those subtask commits linearly.
Its validated checkpoint is `4ffbad8f1ae09bc81f316444cd27433a436c35ab`; the complete tree is identical to pre-rebase `b599fa9`.
All seven required slice heads remain actual ancestors. The 32 focused processor/cache/pool tests and 5 reference-subject tests pass again after rebase.

Canonical identity fixtures execute actual naive RGBA8 intermediates. They distinguish operation keys from output-content identities.
The lifecycle, cache, and pool models specify ownership and publication. Physical allocations and browser workers remain production obligations.
S18 must freeze the shared image source tree alongside spec and bind the reference compiler/dependency inputs.
PR creation remains the last S17 task.
