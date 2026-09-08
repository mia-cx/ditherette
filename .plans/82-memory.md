# S40 browser boundary and repeated-use memory

Issue #82. Worktree `v1-s40-memory`, branch `impl/v1-s40-memory`, starts at
prerequisite join `19240e706c7be91e4f6d88f5470f266e3e6d3fbf`.
Root owns shared browser selection, scripts, CI, and broad conformance.
This task owns only new memory/boundary browser fixtures and this plan.

## Reuse and scope

Use `installed-browser.mjs` unchanged with the explicitly hashed ordinary tarball.
Existing stage-cache/progress fixtures already cover durable outputs, separate
instances, callbacks, copy failures, and disposal. Native preparation tests cover
entry/byte eviction and exact allocated-capacity accounting.
Add browser boundary and sustained-use evidence, not another cache framework.

The fixture observes actual Wasm memory through initialization instrumentation,
restored immediately after each public `createDitherette` call. Wasm page
high-water is not the configured processing-capacity limit or browser heap use.
No release of Wasm pages on `dispose` is promised or asserted.

## TODOs

- [x] Exercise maximum slim source/output axes and logical pixel-limit preflight.
- [x] Exercise bounded repeated changing calls with actual page observations.
- [x] Run the fixed tarball in scalar Chromium, Firefox, and WebKit; record versions,
  budgets, counts, page high-water, absent coverage, and the clean commit.

No runtime, frozen oracle, image, guard, shared fixture, or tarball changes.
No Wasm build or performance measurements. Threaded WebKit remains the recorded
S41 engine-cleanup gate; this scalar fixture makes no threaded claim.

## Validation

`packages/ditherette/tests/memory-browser.test.mjs` uses the installed fixture
unchanged. The new caller-side fixture passes Chromium 147.0.7727.15,
Firefox 148.0.2, and WebKit 26.4. Four test records pass, including the parent.
Node syntax, focused Prettier checks, and `git diff --check` also pass.

The ordinary artifact is
`.worktrees/v1-rows-auto-validation/target/rows-auto-dc81818a-public/ditherette.tgz`,
SHA-256 `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
The fixture checks that digest before offline installation. No package rebuild
or private execution control participates in these browser checks.

Each browser completes ten maximum-axis calls across all five methods, using
32768-by-1 and 1-by-32768 sources with 16384-by-1 and 1-by-16384 resize outputs.
Input allocation stays 128 KiB. Twenty-four validation cases cover side limits,
logical source/output pixel limits, missing source storage at the accepted area
limit, and zero output width. Two maximum-area output requests reject with
`memory-limit` before processing, then the same instance recovers.
This proves shape/preflight behavior, not successful processing of a 67M-pixel image.

Repeated-use checks churn 128 distinct caller contents through Process and direct
quantization. One batch makes 256 warmup calls; two more make 512 checked calls.
Outputs retain correct extents, bytes, warnings, and caller-owned input. A saved
result survives churn and disposal; a separate instance remains usable.
These are caller-behavior checks, not private cache-hit or eviction-count claims.

All three engines report the same actual memory observations:

| Observation | Bytes |
| --- | ---: |
| Boundary instance configured processing limit | 8,388,608 |
| Repeated-use instance configured processing limit | 262,144 |
| Repeated-use memory before warmup | 1,179,648 |
| After warmup and after each repeated batch | 1,245,184 |
| After disposal | 1,245,184 |
| Maximum-axis instance page high-water | 3,145,728 |

The repeated-use memory grows from 18 to 19 Wasm pages, then stays at 19 pages.
Fixed module overhead makes actual pages larger than the processing-capacity
limit. Browser heap, returned JS buffers, and allocator capacity are distinct.
Disposal does not shrink Wasm pages; this test does not assert that it should.

Run with the exact artifact and existing scalar WebKit launcher:

```sh
DITHERETTE_TEST_TARBALL=/home/mia/mia-cx/ditherette/.worktrees/v1-rows-auto-validation/target/rows-auto-dc81818a-public/ditherette.tgz \
DITHERETTE_TEST_TARBALL_SHA256=1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d \
DITHERETTE_TEST_WEBKIT_EXECUTABLE=/home/mia/mia-cx/ditherette/.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit \
node --test packages/ditherette/tests/memory-browser.test.mjs
```

No compiler target was created. All owned browser/test jobs exited.
Root joins this fixture, adds script/CI registration, and owns broad conformance
and the S40 PR. Threaded repeated-use memory is not certified by this scalar test.
