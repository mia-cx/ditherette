# S19 scalar call integration

Read [the approved PRD](../docs/plans/ditherette-v1/README.md) and [S19 acceptance](../docs/plans/ditherette-v1/slices.md#s19).
The coordinator joins reviewed child commits here, validates the actual public call, and opens one unmerged slice PR.
Base branch `impl/v1-s19-base` contains all S02/S06/S18 prerequisites at `1bd175127f92d55fb2ddf693a67d99b9fb190667`.

## TODOs

- [x] Verify the literal-copy checkpoint and join corrected freeze policy plus private factory generation.
- [x] Join the validated private processor and wire its actual Wasm fixtures into the standard scalar build.
- [x] Join the fallible Rust processor and public scalar wrapper after their focused validations.
- [x] Verify actual packed-package calls, input/output ownership, bounded memory, errors, isolation, and disposal.
- [x] Collect one exclusive native nearest experiment after every agent and build exits; retain exact accepted code if the candidate loses.
- [x] Rebase against the recorded immediate base, run invalidated checks, and open the unmerged S19 PR with exact evidence.

## Evidence

- Literal baseline `0ede7f6c` preserves four files byte-for-byte and nearest with one declared import substitution. The coordinator independently verifies all five manifest hashes.
- Corrected policy join `89b570e0` preserves every baseline Rust byte. Full trusted S18 guard passes.
- Private factory delivery `8ecdf786` proves separate mutable glue, memories, and externref tables with the same compiled module. It includes seven focused fixtures and scalar/threaded package builds.
- S18's final documentation head `662d6483` passes CI run `34131890077`; earlier code head `c03c3c85` passes cold-cache run `34131713248`.
- Native trial `s19-nearest-trial-01` passes all ten cases with exact outputs, 8,000 samples, and 80 sequential children reaped. [The measurement report](60-nearest-measurement.md) records artifact identities and actual medians.
- Promotion `964683f46a24c248aa91318bd6280ca91bec88f7` keeps the measured arithmetic unchanged. Its 285 native tests, Wasm compilation, and trusted guard pass.
- Delivery `b48511321f47722fcfb48c51a40a05a825f06a7b` joins the integration branch at `fdef72b5788c3c09fc6439991a01bfc851dc71ce`. Public wrapper/adapter validation remains pending.
- Private adapter `a1ca26f09aa41dc13c29940bb19731b7845279be` joins at `ef36ada38b7b9d2a034be4f2ad754b6297be93ad`. Its final evidence is in `f4f0e86d430880e886db39f27671472170569e9e`.
- Integration runs `pnpm --filter ditherette-wasm test:private` against the standard scalar release artifact. All six actual Wasm fixtures pass, including 512 repeated success/failure cycles without handle or memory growth.
- The trusted S18 checker passes on this integration, including four dependency contexts and five isolation compilations. Frozen content and policy remain unchanged.
- Code join `a9835a97276bfc726590f931caa2af1b4b0b3d9a` includes the public wrapper and packaging correction `9c0738e737f62860e309bd9819541368a8b6668c` with the promoted kernel.
- Both Wasm variants build successfully. Integration passes all 285 native tests, 12 public/validation fixtures, public type checks, two staging fixtures, five factory fixtures, six private Wasm fixtures, Rust formatting, and the trusted freeze check.
- The installed tarball passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4. It resolves actual scalar assets without isolation or threaded requests and verifies all nine nearest anchors, custom inputs, ownership, errors, and disposal.
- WebKit uses the task-local launcher `/tmp/ditherette-webkit-libs.2dS6Yu/webkit`; no host libraries or existing browser binaries change.
- A merge-preserving rebase against the latest `origin/impl/v1-s19-base` leaves this exact code head unchanged. Literal baseline, measured promotion, and prerequisite SHAs remain ancestors.
- Independent review finds no actionable private-processor memory, cleanup, ABI, or canonical-kernel wiring defects. Its focused actual Wasm checks pass without measurements.
- Open non-draft PR #105 targets `impl/v1-s19-base`, with auto-merge disabled. Final agent delivery `26f6ff34629a4d83f34ca58b1b8be25d5088a2b9` adds only provenance and evidence after the validated package code.

## Ownership

The scalar agent owns production contracts, bounded processor state, private Wasm adapter and caught copy helpers.
The package agent owns public TypeScript validation/loading/errors/types and package fixtures.
The nearest agent owns the separate allocation-free candidate and output conformance tests.
The coordinator owns benchmark experiment generation, shared registration, this join, and progress/PR records.

The frozen spec/image closure stays unchanged. Measurements require a quiet phase with all implementation agents and builds drained.
Publishing, tags, merges, deployment, rollout, and non-exact acceptance remain outside scope.
