# S30 complete Process benchmark preparation

Issue #71. Branch `impl/v1-s30-bench` starts from native baseline `3335bb69acc6762a30a0b6844aef436c2e6b8de6`.
Its validated prerequisite join is `22b6dd78`. Existing production kernels remain unchanged.

## Scope

Register actual native/public Process calls and equivalent staged resize plus ditherAndQuantize calls.
Reuse frozen Process reference identities with settings `{palette, recipe}` and output dimensions from `recipe.output`.
Require exact process-versus-staged production equality, including metadata and RGBA8 rounding barriers.
Retain inherited optimized-resize/frozen differences as diagnostics; never alter reference bytes or tolerances to hide them.
Keep copies, preparation, output allocation, and destruction inside each timed complete call.
No TypeScript equivalence claim or new optimization is part of this checkpoint.

## TODOs

- [ ] Add native subjects, typed settings, and worker registration; verify identity, composition, and invalid requests without measurements.
- [ ] Extend public Process/staged adapters, indexed bounds, and protocol tests without package builds.
- [ ] Declare a bounded representative full-pipeline matrix with filter-specific targets and fixed budget; commit and report requirements.

Runtime/package code belongs to the runtime agent. Frozen oracle files belong to the oracle agent.
This worktree alone uses `target/compiler`; no shared compiler cache is assigned.
Fresh package validation waits for the runtime integration. Final joins, fresh role builds, and exclusive measurements belong to the coordinator.
Preserve all evidence and follow completed-PR compiler cleanup when active ownership ends.
