# S25 measurement budget

Measure all-mode baseline `0085972a05a3dbdbbef6d47351d6e37bdd8625d2` against dispatch candidate `230046ff`.
Resolve the candidate's full SHA from Git before preparing artifacts.
Both roles must include the same benchmark protocol and retain their respective production bytes.
The older S24 literal quantizer is not the accepted role for this experiment.

## Cases

- Fifteen complete-call latency cases cover every matching policy, including the five existing Euclidean controls.
- Two native forward-conversion controls cover OKLCH and CIELCH.
- Seven native score-batch controls cover Euclidean, chord, arc, CompuPhase, Rec.601, Rec.709, and CIEDE2000.

Complete calls run natively and through the installed package in Chromium, Firefox, and WebKit.
Use the existing 128×96 fixture and 64-entry palette recipe from S24.
Untimed conformance covers palette sizes, alpha policies, ties, and error boundaries; do not multiply the timing matrix by them.
Standalone metrics use exact f32 score output, without timed conversions or output allocation.
Their arithmetic is unchanged, so component timings are coverage rather than an acceleration claim.

Two alternating AB/BA pairs produce at most 276 workers across all cases.
Each worker uses 20 samples, 50 ms warmup, and a 250 ms measurement cap, retaining at least five valid samples.
Declare complete fixture and tool identities before starting. Use fresh browser collector snapshots after the stability fix.
Drain every agent, build, and test before measuring. Run one worker at a time under the shared lease.

## Decision

Select only exact output, metadata, warning, and score comparisons.
Apply the existing paired gate to every required case, including the 10% regression limit and noise checks.
The candidate should reduce complete-call time for at least one inexpensive metric family without losing another case.
CIEDE2000 spends more time in score arithmetic, so removing dispatch need not produce a measurable gain there.
Keep the exact baseline if the candidate loses or has no demonstrated benefit.
Record raw and compressed Wasm sizes. Any package-size gate still applies.
One candidate comparison is the target; repeat only noisy or inconclusive evidence within the approved slice budget.

No S25 measurement has run. Benchmark score adapters and executable inventory remain to be implemented before artifact preparation.
