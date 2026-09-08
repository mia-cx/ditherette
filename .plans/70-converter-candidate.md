# S29 prepared-converter candidate

Issue #70. Branch `impl/v1-s29-converter` starts from literal public baseline `4e0c134d5c846d32af6b253ced0099993122b251`.
The baseline worktree stays unchanged. This is one candidate revision, not an accepted optimization.

## Scope

Reuse `PreparedQuantizer::converter()` for alpha-prepared target RGB in the Yliluoma pixel loop.
Use the same crate-private read-only accessor signature as S28.
Keep adaptive placement conversion, exhaustive pair/ratio order, ties, alpha, and all arithmetic unchanged.
Retain existing capacity accounting, including temporary placement conversion. Add no mixture table or memo.

## TODOs

- [ ] Replace only target conversion construction and verify native exactness, budgets, and unchanged shared behavior.
- [ ] Declare a bounded typed native/public benchmark plan; verify its cases and caps without measurements.
- [ ] Build scalar/threaded artifacts and verify private/interface/installed three-engine conformance; record identities and stop.

## Measurement declaration

Compare against fresh literal `4e0c134`, using two alternating pairs and at most 20 samples per case.
Each worker has a declared 10-second measurement cap. Keep complete-call latency separate from throughput.
Use a small explicit case list across palette sizes, working spaces, matrix sizes 2/4, and one tiny size16 control.
Do not take a Cartesian product or time the 367-case conformance fixtures.
Only the coordinator runs measurements after all agents drain. No PR, issue, release, or spec writes.

Native/scalar/threaded builds use only the assigned S22 cache and its variant children.
The baseline evidence, independent frozen-Wasm fixtures, and seven target-local differences remain in `70-yliluoma.md`.
