# S40 integrated package conformance

## Scope

Validate the joined package and website without changing production algorithms.
Base `19240e706c7be91e4f6d88f5470f266e3e6d3fbf` contains the validated S35/S36/S37 runtime,
ordinary automatic-policy fixtures, and S39 `5938b248506ae14d24471498c3a90bb42ed3c32a`.
Join the final publication heads before delivery.

## TODOs

- [ ] Add crate-owned conformance commands and CI using existing installed-package fixtures.
- [ ] Join focused scalar boundary and repeated-memory browser checks from the isolated memory branch.
- [ ] Run the integrated native/interface/browser/website checks and record exact artifact evidence.
- [ ] Record failed or absent coverage, file an unmerged PR, and return compiler outputs for cleanup.

## Ownership

Root owns CI, command orchestration, shared browser selection, integration, and delivery.
The memory agent owns two new package browser fixtures and its own plan in `v1-s40-memory`.
The PR agent owns the S35/S36/S37 delivery branches. The S41 agent does read-only preparation.
No benchmark runs during this work.

## Evidence and limits

The joined crate/package/build inputs match ordinary source `dc81818a7ea244ccfcda1c2d88b9df3592fd8dd1`.
Its tarball SHA-256 is `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
The full trusted frozen guard passed. Both Chromium and Firefox passed nine actual automatic-policy cases.
These earlier results cover unchanged inputs, not newly added conformance fixtures.

Pinned WebKit 26.4 retains atomic-wait workers in the independent S34 diagnostic.
Keep its threaded cleanup gate failed until a verified engine fixes it. Scalar WebKit remains required.
The CI report must distinguish this omitted unsafe threaded execution from passing scalar coverage.
Performance gates, inherited bilinear reference drift, and noisy comparisons remain S41 work.
