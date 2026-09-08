# S40 integrated conformance reuse

Start from the explicit S35/S36/S37/S39 join. Read S40 and the approved execution contract.
Keep the frozen oracle, landed kernels, and existing fixture assertions unchanged.
Test the final installed tarball by digest. Historical checks do not certify a changed artifact.

`packages/ditherette/tests/installed-browser.mjs` already installs an explicit tarball offline,
checks its optional required SHA-256, serves declared assets, and closes each browser in finally.
The broad tarball, stage-cache, and progress fixtures cover existing methods, durable results,
separate instances, copy failures, callback reentry, disposal, and recovery.
S34 adds actual nested-worker lifetime observation, custom threaded inputs, and partial-start failure checks.
Extend these fixtures for missing integrated cases. Do not create another browser installation/server framework.

Private Wasm fixtures already inspect actual memory pages and binding-table slots during repeated failures.
Nearest and quantize include 512-cycle checks; Process checks input/result failure and recovery across 64 cycles.
These private Node checks are useful evidence, not browser proof or evidence for all maximum dimensions.
Use mode-specific boundary requests and observable preflight rejection without allocating impossible test images.
Distinguish planned allocation capacity, Wasm page high-water, and browser-owned pool resources in the report.
Inspect existing production cache and preparation tests before adding eviction or budget fixtures.

Only the frozen-spec workflow currently exists under `.github/workflows` at S34.
Add the required package conformance CI using existing crate/package scripts and pinned build ownership.
Do not replace the trusted guard or add publishing to the conformance job.

Pinned WebKit 26.4 fails actual atomic-wait worker cleanup in S34, independently of Ditherette and Rayon.
Keep [the recorded release gate](https://github.com/mia-cx/ditherette/issues/83#issuecomment-5589175119) visible.
Verify a recorded engine containing the upstream fix before claiming threaded cleanup passes there.
Report absent or failed coverage explicitly. No external consumer project is required.
