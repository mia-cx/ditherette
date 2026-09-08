# S40 integrated conformance

The integrated package passes scalar Chromium, Firefox, and WebKit conformance.
Chromium and Firefox also pass actual threaded ownership, failure, and host-termination checks.
This is implementation evidence, not publication or rollout approval.

## Source and artifact

Dependency base `5fccb9e6a51c6de49fd0051b204fb16bfde75e22` contains these unmerged heads:

| Slice | PR | Head |
| --- | --- | --- |
| S35 | [126](https://github.com/mia-cx/ditherette/pull/126) | `6bbe113b99a08f2a11ade6296ddf7f25e32e041b` |
| S36 | [127](https://github.com/mia-cx/ditherette/pull/127) | `b542bd94a5dbc724de73815ae0008a22985147fd` |
| S37 | [128](https://github.com/mia-cx/ditherette/pull/128) | `91b114ba610588c504a7551e8123d72e36eb9e66` |
| S39 | [125](https://github.com/mia-cx/ditherette/pull/125) | `5938b248506ae14d24471498c3a90bb42ed3c32a` |

Conformance implementation checkpoint `b6e78a63` adds tests and CI only.
The ordinary package comes from source `dc81818a7ea244ccfcda1c2d88b9df3592fd8dd1`.
Its SHA-256 is `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
Runtime source, Cargo inputs, compilers, factory generation, package source, and staging code remain unchanged.
The crate manifest adds test commands only. Existing artifact provenance stays attached to its original source.
The previously passing full trusted guard covers the unchanged frozen checkpoint and build policy.

## Local results

| Checks | Result |
| --- | --- |
| Native release, scalar / threaded | 424 / 425 pass |
| Public interface and types | 42 tests and TypeScript checks pass |
| Generated factories / private ownership / staging / engine selection | 4 / 17 / 2 / 1 pass |
| Broad scalar package, all three engines | Pass |
| Main-JS preferred fallback and required capability errors, isolated and non-isolated | Pass in all three engines |
| New scalar boundaries and repeated memory | Pass in all three engines |
| Actual nested workers, independent pools, partial-start cleanup, host termination | 12 test records pass in Chromium/Firefox |
| Website cancellation, stale results, and faithful fallback | 68 server and six Chromium tests pass |

The broad fixture retains 54 convolution cases, 27 trilinear cases, 15 matching tags,
110 field vectors, 1,650 separable compositions, 360 diffusion vectors, and 423 Process compositions per engine.
It also retains custom input, durable output, independent instance, cache-stage, callback, and copy-failure checks.
Each engine checks 367 target-local frozen Yliluoma vectors and 734 untimed adapter calls.
These adapter calls are correctness checks, not performance samples.

The unchanged automatic-policy fixture already passed 23 test records on this exact ordinary package.
Nine cases per Chromium/Firefox engine compare scalar and actual required-threaded outputs and metadata.
CI includes that fixture; S40 does not claim another local measurement or duplicate browser run.

Browser versions are Chromium `147.0.7727.15`, Firefox `148.0.2`, and WebKit `26.4`.
Local checks use Node `24.19.0`, pnpm `11.13.0`, and pinned Playwright `1.59.1`.
The first broad run passes Chromium/Firefox but cannot launch WebKit because a host library alias is absent.
A WebKit-only retry passes through the existing test-owned launcher. Shared browser files remain untouched.
This launch failure and its retry remain in separate logs.

## Memory and boundary scope

[The focused memory report](../../../.plans/82-memory.md) records the complete fixture and observations.
Each engine runs ten slim maximum-axis calls across all five methods, 24 dimension rejections,
and two maximum-area requests that fail memory preflight. It does not allocate a maximum-area source image.

After 256 warmup calls, 512 changing Process/quantize calls hold scalar Wasm memory at 19 pages, or 1,245,184 bytes.
The configured processing-capacity limit is 262,144 bytes. These quantities differ because pages include module/runtime overhead.
Disposal preserves earlier outputs and does not shrink Wasm pages. The separate instance remains usable.
Native preparation/stage tests cover entry and byte eviction. Browser behavior does not independently prove cache hits or eviction counts.

## Repeatable commands and CI

The Rust workspace owns `test:conformance`, `test:conformance:threads`, and `test:conformance:automatic`.
The new package conformance workflow builds one ordinary tarball with the existing fresh-artifact preparer.
It generates the independent native Yliluoma identities and tests the installed tarball by digest.
It runs the native, public/private, scalar browser, supported threaded, automatic-policy, and website checks.
It uploads the tarball, provenance, and browser logs without publishing or tagging anything.
The separate trusted frozen-reference workflow remains unchanged.

Set these paths to the retained artifacts for a local browser rerun:

```sh
export DITHERETTE_TEST_TARBALL=/absolute/path/to/ditherette.tgz
export DITHERETTE_TEST_TARBALL_SHA256=1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d
export DITHERETTE_BENCH_ORACLE=/absolute/path/to/public/scripts/oracle
export DITHERETTE_BENCH_YLILUOMA_FIXTURES=/absolute/path/to/yliluoma-conformance.json
pnpm --filter ditherette-wasm test:conformance
DITHERETTE_TEST_ENGINES=chromium,firefox pnpm --filter ditherette-wasm test:conformance:threads
```

Optional `DITHERETTE_TEST_CHROMIUM_EXECUTABLE`, `DITHERETTE_TEST_FIREFOX_EXECUTABLE`, and
`DITHERETTE_TEST_WEBKIT_EXECUTABLE` select test-owned runtimes. Omitted engine selection requires all three.
An empty, duplicate, or misspelled engine list fails instead of passing an empty suite.
The automatic fixture takes `DITHERETTE_BENCH_AUTO_BUNDLE` pointing to the ordinary `bundle-source.json`.

Local logs remain under `v1-s40-conformance/target`: `native-scalar.log`, `native-threads.log`,
`interface.log`, `glue.log`, `private.log`, `conformance-scalar.log`, `conformance-webkit.log`,
`conformance-memory.log`, `conformance-threads.log`, and the website logs.
The earlier ordinary build, full guard, and automatic-policy records remain with their original artifacts.

## Open release gates

Pinned WebKit's actual atomic-wait worker cleanup still fails in the independent S34 diagnostic.
S40 does not launch unsafe threaded WebKit tests or claim that coverage passes.
The CI job reports this omission explicitly. S41 retains the reproducible engine gate.
Scalar WebKit remains required and passes.

Full-call performance, equivalent TypeScript comparisons, inherited reference differences,
and noisy required measurements remain S41 work. Browser heap limits and successful maximum-area processing
are not established by the bounded-memory fixture. No external consumer project participates.
The workflow is locally syntax-checked; its first GitHub execution is separate from these local results.
