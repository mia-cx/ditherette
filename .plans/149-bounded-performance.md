# Bounded performance pass

## Scope

Start from merged main `9f85ff4e`. Measure scalar native spec/production and real-browser JS/Wasm before selecting improvements. Cover every registered algorithm with representative cases, then use scale and setting permutations for complete calls. Optional threading is secondary; scalar remains required.

Frozen spec and shared image files stay unchanged. Preserve the landed optimized resize kernels. Keep anti-aliased bilinear; label its historical JS downscale comparison as a quality mismatch. Historical JS area is deprecated, not a like-for-like target. Approximate output changes need Mia's visual acceptance; prefer exact changes.

## Tasks

- [x] Inventory benchmark coverage, retained evidence, build prerequisites, and execution commands.
- [x] Prepare fresh recorded artifacts and a bounded baseline matrix outside Git.
- [x] Run one quiet benchmark campaign covering native kernels and browser complete calls.
- [x] Rank comparable losses by absolute cost and likely benefit. Select at most three high-value areas for this pass.
- [x] Try at most two focused candidates per selected area. Keep only verified improvements; stop an area after two unsuccessful candidates.
- [x] Remeasure affected cases and broad controls. Report wins, losses, noise, coverage gaps, and remaining release requirements.
- [x] File measured changes in scoped PRs; update progress and remove disposable compiler targets.

## Measurement rules

Use the existing ditherette-bench coordinator, verifier, and exclusive lease. No overlapping benchmarks, builds, tests, or agent implementation during measurement. Drain agents first. Wait for process exit; do not poll logs or counters.

Separate first-image processing, changed-settings recomputation, and identical-call cache hits. Keep module initialization separate where the existing runner permits. Use fresh interleaved comparisons to limit thermal bias. Browser timing must use optimizing Firefox, not its known baseline-only debugger path.

Use a representative screening matrix before expanding expensive cases. Do not shorten minimum samples or relax established correctness gates to claim success. Remeasure noisy material results once; report unresolved noise after that. Package size accompanies any authorized speed-build experiment; `opt-level=s` remains the default until a measured selection is approved.

Historical reports guide selection but do not count as current-head measurements. Raw fixtures, artifacts, profiles, and benchmark results stay outside Git. The broader #149 target remains open unless every eligible required case meets it; an average speedup cannot close it.

## Ownership

The coordinator owns this plan, artifact preparation, benchmark scheduling, and final verification. Three read-only agents inventory runner commands, historical performance losses, and algorithm coverage. Each stops after its report. Candidate ownership will be assigned only after the baseline identifies worthwhile work.

## Completion

Deliver measured coverage and the worthwhile exact improvements within these bounds, not an unbounded search for maximum speed. Keep publication, rollout, and legacy retirement held. Explicitly distinguish finishing this pass from satisfying every release performance gate.

## Measured outcome, 2026-09-21

PR #172 fixes randomized autocfg probe identities. PR #173 keeps shared immutable color tables, exact saturating byte-feedback conversion, and explicit CIELAB reconstruction inlining. Both remain unmerged. Child issues #170 and #171 track these changes under #149.

The three selected areas were color setup, Lanczos resizing, and diffusion. Constant-radius Lanczos specialization and tap hoisting showed no useful gain and were removed. The second diffusion candidate and color-table reuse passed exact verification. Explicit CIELAB inlining removed a compiler-induced inverse-color regression.

Measured source is `8b3be973a8a3eb274cbf1cef1f95df974c7c7020`. Delivery `cd8e13e1d88ea6c78b8d97214b9828bc908ff509` has identical implementation and build-script contents. Full tables, retained noise and inherited differences, fixtures, artifacts, and raw evidence remain outside Git at `/home/mia/mia-cx/ditherette-performance-20260920`.

Finished task-owned compiler targets are removed. Recorded artifacts and executable copies of the lease and plan helpers remain outside Git for reproduction.

WebKit diffusion and some Lanczos cases still lose to JS. Issue #149 remains open. Publication, website-default rollout, and legacy retirement remain held.
