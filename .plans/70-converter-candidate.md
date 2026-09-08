# S29 prepared-converter candidate

Issue #70. Branch `impl/v1-s29-converter` starts from literal public baseline `4e0c134d5c846d32af6b253ced0099993122b251`.
The baseline worktree stays unchanged. This is one candidate revision, not an accepted optimization.

## Scope

Reuse `PreparedQuantizer::converter()` for alpha-prepared target RGB in the Yliluoma pixel loop.
Use the same crate-private read-only accessor signature as S28.
Keep adaptive placement conversion, exhaustive pair/ratio order, ties, alpha, and all arithmetic unchanged.
Retain existing capacity accounting, including temporary placement conversion. Add no mixture table or memo.

## TODOs

- [x] Replace only target conversion construction and verify native exactness, budgets, and unchanged shared behavior.
- [x] Declare a bounded typed native/public benchmark plan; verify its cases and caps without measurements.
- [ ] Build scalar/threaded artifacts and verify private/interface/installed three-engine conformance; record identities and stop.

## Measurement declaration

Compare against fresh literal `4e0c134`, using two alternating pairs and at most 20 samples per case.
Each worker has a declared 10-second measurement cap. Keep complete-call latency separate from throughput.
Use a small explicit case list across palette sizes, working spaces, matrix sizes 2/4, and one tiny size16 control.
Do not take a Cartesian product or time the 367-case conformance fixtures.
Only the coordinator runs measurements after all agents drain. No PR, issue, release, or spec writes.

Native/scalar/threaded builds use only the assigned S22 cache and its variant children.
The baseline evidence, independent frozen-Wasm fixtures, and seven target-local differences remain in `70-yliluoma.md`.

## Candidate implementation

The production diff adds S28's read-only `converter()` borrow and replaces the Yliluoma target conversion call.
All other pixel-loop statements, helpers, capacity charges, and kernel code remain unchanged.
Twenty-three native tests pass across `prod_yiluoma`, `prod_processor_fields`, `prod_processor_quantize`, and `prod_quantize`.
This includes 367 native fixture requests, exhaustive matching/tie coverage, exact/one-under budgets, caught failures, and shared converter semantics.

## Bounded plan

Production candidate is `abe8241`. `yliluoma_converter_plan` declares eight native and eight public cases, not a Cartesian product.
Two pairs across native and three browsers produce 128 serial workers, with at most 20 samples and a 10-second cap each.
The cases cover 32x24 sRGB/palette2/size2, 8x8 linear/palette16/size2, and 16x12 Oklab/palette8/size4.
They also cover 8x8 CIELAB/palette16/size4, 4x4 CIEDE2000/palette4/size4, and 8x8 adaptive OKLCH/palette8/size4.
The large-palette control is only 1x1 with 256 entries and size2. The size16 control is only 2x2 with two entries.
Full-call source copies and preparation remain in each timed operation; public setup uses primed instances and no application cache.
The generator test verifies case identities, dimensions, sample limits, worker count, exactness, and declared preparation.
Both existing typed adapter tests and the generator test pass; all benchmark targets typecheck.
The generator runs only on this candidate branch. The accepted baseline needs no generator edit.

A separate review found the literal baseline's Rust browser worker omitted Yliluoma from indexed-output classification.
The coordinator assigned that benchmark-only repair to S28. Join its verified commit before coordinator measurements.
Installed package and JavaScript-adapter correctness checks do not by themselves cover that Rust worker path.
