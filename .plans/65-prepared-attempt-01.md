# S24 prepared attempt 01

## Status

Prepared, unmeasured, and paused for the timed-output stability review.
Do not run these artifacts as final S24 evidence. The public retention fix requires fresh artifacts.
Native policy now explicitly accepts checked endpoints only for the fixed deterministic in-process callables.
It does not claim observation of every timed output or transient A/B/A detection.

All paths below are relative to `.worktrees/v1-s24-bench/crates/ditherette-bench/target/s24-quantize/`.
The coordinator owns quiet clearance and all future measurements. No result directories exist for this attempt.

## Sources and scopes

Literal source `a23260edec0452fd17c13073636f548b07804230` receives only benchmark adapters at accepted artifact source `ccb9bceb28563c562dd5e6c05f68c056c18e3519`.
Both commits have production tree `d4847ee149dfa346a85a23f8be28e6c9eb1fb768`.
Production, spec, image, and freeze-policy bytes compare identical with Git.
Candidate artifact source is `b2064ff29eeaf7d922c84cd927fb4e7829cceac8`, including validated public quantize `0ae8b95f`.

Native accepted `baseline:quantize:request:literal` calls the old full-call convenience function.
Candidate `candidate:quantize:request:prepared` calls the budgeted convenience function.
Both borrow source bytes and time validation, preparation, conversion, matching, owned output construction, and disposal.
Five packed-forward controls prepare tables/output storage outside timing and observe each conversion result through the same optimizer barrier.
Exact numeric verification and diagnostic inverse rendering are untimed.
Native conformance checks before/after outputs under the documented deterministic-kernel assumption.
The typed registry cannot certify arbitrary stateful subjects. Result disposal remains inside the full-call scope.

Public accepted and candidate use the identical package-capable candidate artifact.
These are same-artifact package controls, not optimization evidence or a pre-optimization public baseline.
Calls use primed instances with no application cache, no benchmark-only hashing, and no TypeScript quantize claim.

## Files and SHA-256

| File | SHA-256 |
| --- | --- |
| `native-accepted/ditherette-bench` | `3a888084f2c9f9bea610eef4909100492816111034b99b826fd6bc4e6a316916` |
| `native-candidate/ditherette-bench` | `036388a412dbb3eb06bce9b072d73c294f2bed1927632d630cee5af41ab8b83f` |
| `ditherette-bench-pair` | `b7cdf4e07b1a12ccf0b0c86476404b885fb30296407b6ba7c94320179c224efc` |
| `public-build/ditherette.tgz` | `39f607810adce816973ad4adb0301607281698622341cd72e6edc02fcb0d3eee` |
| `public-build/build-provenance.json` | `7a7aad7c239ff558204ca9ac67e03c62a0b99a1afe3f9366002c106116f053e9` |
| `conformance.json` | `ea98f1b8379afc434cbeae19d3822eab36b83d7d1fa9e1351559935d20c3dd60` |
| `prepared-native/prepared.json` | `67a71eef5e2affa9a0341fb458ba1491215b3b68a2e2f0f109850eac3ccd288d` |
| `prepared-chromium/pair/prepared.json` | `5e485e10f8e90cfd08bcd1ae1642536a597df0dc466ba53a462ce63f7f72bb09` |
| `prepared-firefox/pair/prepared.json` | `99d58c3a61c666c73d7d40de815f088d5491bfa5be7dd948ce59fd19bc84be44` |
| `prepared-webkit/pair/prepared.json` | `7b5c389ae4dd40a8c32354ae1c03bb067e74aeb9e2ea159ff2bda6c88239689a` |

Each browser directory contains complete package, TypeScript, transport, browser, and Playwright snapshots plus source/build provenance.
The sources reuse S23's validated runtimes, including the alias-preserving relocated WebKit install.
Node is v24.19.0; Playwright is 1.59.1. All engines are headless and isolated with no extra launch arguments.
Prepared snapshots no longer depend on the mutable source checkout. Rebuilding requires checking out the recorded artifact revision.

## Budget and validation

The unchanged declaration contains 13 native and six public cases per engine, two AB/BA pairs, and at most 124 serial workers.
Each case uses a 128×96 fixture, 20 samples, 50 ms warmup, a 250 ms cap, and a 2 ms throughput target.
The preparation notes do not attest that the host is quiet.

Native accepted validation passes ten focused tests and the trusted S18 guard.
Candidate validation passes 56 Rust tests, Wasm compilation, both crate formatting checks, and the same trusted guard.
Ten controlled Node protocol tests pass.
The actual installed-package benchmark adapter passes all three engines with 17 frozen fixtures and primed/fresh preparation.
The browser suite checks all five matching spaces, three alpha policies, palette warnings, exact indexed metadata, and durable output ownership.
No operation runs under a benchmark measurement timer during this validation.

The trusted guard retains revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and digest `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
