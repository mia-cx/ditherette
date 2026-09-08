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
- [x] Build scalar/threaded artifacts and verify private/interface/installed three-engine conformance; record identities and stop.

## Measurement declaration

Compare against fresh literal `4e0c134`, using two alternating pairs and at most 20 samples per case.
Each worker has a declared 10-second measurement cap. Keep complete-call latency separate from throughput.
Use a small explicit case list across palette sizes, working spaces, matrix sizes 2/4, and one tiny size16 control.
Do not take a Cartesian product or time the 367-case conformance fixtures.
Only the coordinator runs measurements after all agents drain. No PR, issue, release, or spec writes.
Seek a lower complete-call median on the conversion-heavy palette2 case; reject confirmed per-case regressions above 10%.
Keep the literal implementation if gains are absent or required evidence remains incomplete.

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

## Final validation and retained identities

Literal production/package revision remains `4e0c134d5c846d32af6b253ced0099993122b251`.
The accepted benchmark baseline is now `092f2dd0499ed0c5c3784e6533d726d25c7e142b`, which only repairs worker classification and records evidence.
Candidate production revision is `abe8241`; candidate plan revision is `1151c85`.
The identical worker repair is cherry-picked as `b949d308533dae43299e9a6d9b74d37daaa2a1ef`.
Accepted and candidate worker code/tests have no diff. All seven worker tests and the generator test pass after joining it.
The final production diff against accepted contains only the two-file converter borrow/call change.

Both release builds pass. The candidate passes 23 native tests, 12 private tests, and all 25 public interface/type tests.
All benchmark targets typecheck; both typed Yliluoma adapter tests and the bounded generator test pass.
Installed Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 each pass 367 frozen-Wasm vectors and 734 untimed actual JavaScript-adapter calls.
The same runs retain the existing 91 field vectors and 1,365 compositions per engine.
Exact/one-under budgets, caught failures, ownership, reentry, disposal, alpha, placements, and every accepted metric remain covered.

The full guard executes from trusted S18, including policy/content/compiler/dependency checks and native/Wasm semantic isolation.
Threaded production isolation also passes. It retains revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and SHA-256 `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
S18 cache ownership returns to the coordinator after the guard exits.

| Artifact | Literal bytes | Candidate bytes | Candidate SHA-256 |
|---|---:|---:|---|
| Scalar Wasm | 241,988 | 241,989 | `6f4e28a14180f64ecbbd1f2e0a30935128fbb7cfa209c2f2a0fc40e26717bf77` |
| Threaded Wasm | 331,861 | 331,853 | `0ce8329359580fc9f9aae5787d1e03733e2d52b324e30838f6704dfc88621e5d` |

Build artifacts remain in this worktree's ignored `crates/ditherette-wasm/dist` and staged package distribution.
The literal baseline's artifacts remain untouched. The coordinator separately owns reusable frozen-Wasm oracle protocol integration.
This candidate is exact in current conformance, not performance-selected. No measurement, PR, issue, or release action runs here.
Stop and drain after pushing this checkpoint; the coordinator owns the next measurement phase.
