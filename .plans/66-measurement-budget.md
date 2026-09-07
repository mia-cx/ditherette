# S25 measurement budget

Measure all-mode baseline `0085972a05a3dbdbbef6d47351d6e37bdd8625d2` against dispatch candidate `230046ff` for new modes.
For the five existing Euclidean modes, compare the delivered S24 parent against that candidate instead.
This checks parent regressions directly, rather than treating the unselected all-mode baseline as already accepted for existing modes.
Resolve the candidate's full SHA from Git before preparing artifacts.
Both roles must use compatible verified benchmark protocols and retain their respective production bytes.
The older S24 literal quantizer is not the accepted role for this experiment.

## Cases

- Fifteen complete-call latency cases cover every matching policy, including the five existing Euclidean controls.
- Two native forward-conversion controls cover OKLCH and CIELCH.
- Seven native score-batch controls cover Euclidean, chord, arc, CompuPhase, Rec.601, Rec.709, and CIEDE2000.

Complete calls run natively and through the installed package in Chromium, Firefox, and WebKit.
Prepare separate parent-control and new-mode experiments because their accepted artifacts differ.
The five parent controls consume 80 workers. Ten new modes, two conversions, and seven metric controls consume 196 workers.
Use the existing 128×96 fixture and 64-entry palette recipe from S24.
Untimed conformance covers palette sizes, alpha policies, ties, and error boundaries; do not multiply the timing matrix by them.
Standalone metrics use exact f32 score output, without timed conversions or output allocation.
Their arithmetic is unchanged, so component timings are coverage rather than an acceleration claim.

Two alternating AB/BA pairs produce 276 workers across all cases.
Each worker uses 20 samples and 50 ms warmup, retaining at least five valid samples.
The measurement cap is 250 ms, except complete CIEDE2000 calls use 10,000 ms.
CIEDE2000 scores every palette entry with more arithmetic; its cap allows five valid samples without shrinking the fixture.
This exception is declared before any measurement. It does not add workers or establish a performance expectation.
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

## Interrupted preparation and one fresh restart

Attempt 01 started two workers. The accepted S24 worker completed 20 samples.
The candidate rejected its stale embedded revision before timing; both workers exited and were reaped.
Those incomplete timings remain in `target/s25-trial-01` and cannot become a fresh baseline.

Shared native build output reused an earlier build-script revision despite the current clean source checkout.
The restart requires local-package recompilation and an untimed executable identity check before preparing pairs.
One complete fresh 276-worker comparison is authorized after that infrastructure repair.
The resulting slice ceiling is 278 launched workers, including the two interrupted workers.
No algorithm candidate, fixture, sample count, gate, or case matrix changes for this restart.
If preparation fails again, retain incomplete evidence and investigate before launching another measurement.
