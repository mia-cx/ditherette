# S19 scalar call integration

Read [the approved PRD](../docs/plans/ditherette-v1/README.md) and [S19 acceptance](../docs/plans/ditherette-v1/slices.md#s19).
The coordinator joins reviewed child commits here, validates the actual public call, and opens one unmerged slice PR.
Base branch `impl/v1-s19-base` contains all S02/S06/S18 prerequisites at `1bd175127f92d55fb2ddf693a67d99b9fb190667`.

## TODOs

- [x] Verify the literal-copy checkpoint and join corrected freeze policy plus private factory generation.
- [ ] Join the fallible Rust processor and public scalar wrapper after their focused validations.
- [ ] Verify actual packed-package calls, input/output ownership, bounded memory, errors, isolation, and disposal.
- [ ] Collect one exclusive native nearest experiment after every agent and build exits; retain exact accepted code if the candidate loses.
- [ ] Rebase against the recorded immediate base, run invalidated checks, and open the unmerged S19 PR with exact evidence.

## Evidence

- Literal baseline `0ede7f6c` preserves four files byte-for-byte and nearest with one declared import substitution. The coordinator independently verifies all five manifest hashes.
- Corrected policy join `89b570e0` preserves every baseline Rust byte. Full trusted S18 guard passes.
- Private factory delivery `8ecdf786` proves separate mutable glue, memories, and externref tables with the same compiled module. It includes seven focused fixtures and scalar/threaded package builds.
- S18's final documentation head `662d6483` passes CI run `34131890077`; earlier code head `c03c3c85` passes cold-cache run `34131713248`.

## Ownership

The scalar agent owns production contracts, bounded processor state, private Wasm adapter and caught copy helpers.
The package agent owns public TypeScript validation/loading/errors/types and package fixtures.
The nearest agent owns the separate allocation-free candidate and output conformance tests.
The coordinator owns benchmark experiment generation, shared registration, this join, and progress/PR records.

The frozen spec/image closure stays unchanged. Measurements require a quiet phase with all implementation agents and builds drained.
Publishing, tags, merges, deployment, rollout, and non-exact acceptance remain outside scope.
