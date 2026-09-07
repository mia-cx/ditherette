# #58 Complete the five-method reference processor

## Summary

Compose the completed references before S18 freezes them. Keep every PR unmerged.

## Validated prerequisite join

Base branch `impl/v1-s17-base` at `4f4b48a04c0ccee51222b7e621ff4a566a495613` contains S05, S10, S11, S13, S14, S15, and S16.
Its 226 listed native tests, Wasm compilation, and formatting checks pass.
The join connects the corrected S14 blue-noise tile to S13's field dispatch.

## TODOs

- [ ] Compose all five typed methods and test both public equalities, metadata, warnings, input preservation, and durable results.
- [ ] Complete strict tagged requests and executable lifecycle, cache/memory, and partition reference models before freeze.
- [ ] Register callable benchmark subjects for every completed reference and reconcile the exhaustive export inventory.
- [ ] Validate the full join, update the stack ledger, and open an unmerged S17 PR against the verified base.

## Ownership

The coordinator owns `spec/pipeline/`, its tests, `spec/mod.rs`, integration joins, this plan, `slices.md`, and the ledger.
The strict-request agent owns existing enum-use corrections and request validation fixtures in its isolated worktree.
The inventory agent audits executable adapters and partition coverage before receiving a bounded implementation assignment.
The benchmark agent owns benchmark subject registration and adapter fixtures in its isolated worktree.

## Gates

No production optimization or spec freeze occurs here. Benchmark registration runs tests without measurements.
Only the coordinator grants a quiet measurement phase after all implementation and build/test work exits.
The first S06 control trial rejected its candidate; it does not establish accepted production performance.

## Evidence

Pending S17 implementation and aggregate validation.
