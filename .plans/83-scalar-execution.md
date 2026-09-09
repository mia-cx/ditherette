# S41 scalar spec and production measurements

Mia authorizes completing the scalar spec/prod optimization workflow under #83.
The benchmark adapter baseline is `2b9bbd68`; the initial converter candidate is `086bbd47`.
Both contain the same corrected collector and 101-case generator. Neither enables native threads.

## Scope and budget

1. Measure all 101 declared cases with frozen spec as accepted and current production as candidate.
   Both roles use the freshly built baseline binary. Retain every ratio, including slower production and inherited resize differences.
2. Measure previous production against converter reuse for affected placement, perturbation, diffusion, and Yliluoma cases.
   Include tiny/zero-strength controls and unchanged forward/quantize/resize controls. Use the same declared case settings.
3. Keep demonstrated exact improvements. Repeat only inconclusive required selection cases once, with both fresh roles and unchanged settings.
   At most two candidate revisions are available for a demonstrated implementation problem. Retain rejected evidence.
4. Measure selected production against spec for the full matrix, then publish the ratios and stacked PRs.

Each case has two alternating AB/BA pairs, four sequential workers, 50 ms warmup, and 5 to 20 samples per worker.
The 250 ms accumulated measurement target applies after five samples. Expensive calls can exceed it; samples are never padded.
The full matrix has 404 workers. Components use preallocated outputs; complete indexed operations include preparation, allocation, and destruction.
The generator and `crates/ditherette-bench/PAIRED.md` define each exact boundary and workload.
No browser or TypeScript timing determines native scalar selection. Browser correctness and release requirements remain separate holds.

## Candidate targets

The candidate removes repeated construction of existing color tables. Conversion equations and field/diffusion/mixing order stay unchanged.
Target at least 20% lower latency for nontrivial field/placement work and at least 10% for a Yliluoma recipe.
These targets apply to fresh previous-production comparisons, not an inferred multiplication of historical speedups.
Every affected required selection case must satisfy the existing exactness, 10% regression, and pair-noise rules.
Unchanged kernel controls diagnose comparison noise; they do not justify rewriting landed code.
Trilinear, nearest, and convolution receive measured spec/prod ratios without a forced new optimization target.

## Ownership and retention

The coordinator builds each clean artifact with the existing native preparation helper, adding only Cargo `--jobs 2` concurrency.
Source inventories, compiler identities, executable hashes, and full revisions bind both artifacts.
All agents and owned builds/tests drain before the shared benchmark lease. Wait for process exit without progress polling.
Check at least 12 GiB free before a run. Preserve reports, events, raw sample records, input recipes, and mismatch PNGs outside target.
Clear complete finished target trees after verified retention. Do not delete sources or alter the frozen spec, image, or guard.
All PRs remain unmerged; no publishing, rollout, deployment, or visual-drift acceptance occurs.

## Progress

- [x] Audit callable coverage and add matched scalar timing adapters.
- [x] Prepare exact converter-reuse candidates and focused correctness evidence.
- [ ] Build clean baseline and candidate artifacts.
- [ ] Run and retain the baseline spec/prod matrix.
- [ ] Measure and select exact production improvements.
- [ ] Publish measured per-kernel results and stacked PRs; clean finished outputs.
