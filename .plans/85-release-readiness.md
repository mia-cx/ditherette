# S43 release-readiness report

Release remains blocked. The S41/S42 join passes the integrated checks below, but required performance, exactness, size, and activation gates remain open.

## Source and artifact

Build source `c43cea1269fcd666835d41c07d82a1c451604107` contains S41 PR131 at `d09e32df8e06ddddec3d6d374c6f22c280bc4d4b` and S42 PR130 at `db78ca0adf60d1a057d10c2bbe7aabc4c1781374`. Both are ancestors. The separate stack audit owns the complete issue/PR/base/head/dependency inventory; root joins it before filing S43.

Fresh ordinary `ditherette@0.1.0` tarball SHA-256 is `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90`. All 48 installed files and raw/gzip/Brotli sizes equal the [S42 artifact](84-package-report.md). The clean build verifies all 1,286 source files before and after preparation.

Validation revision `f331ffcfde354726d8f1266e3839dbc6bffef1e0` adds five test-fixture lines to stage and serve S41's indexed-wire dependency. The original installed-tarball conformance command fails importing the frozen oracle page without those registrations. It passes after the repair. This revision changes no package/build/runtime input; it is not the tarball's build source. Later report commits do not replace either identity.

[Machine evidence](85-artifact.json) records source, artifact, oracle, fixture, and retained S41 report hashes. The tarball and complete build provenance remain in this worktree's ignored `benchmark-results/s43-artifact/`, outside finished targets.

## Fresh integrated checks

All listed checks pass without skips or performance measurements.

| Check                                                                                                                     | Result                                    |
| ------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Native benchmark library, browser-worker/paired-browser/paired-quantize protocols, fixed release generator                | 49 tests                                  |
| Startup role policies, progress, stage cache, indexed wire, timing collector, actual TypeScript closure, release contract | 40 tests                                  |
| Public interface/types, staging, generated scalar/threaded glue                                                           | 42 / 2 / 4 tests                          |
| Website cancellation, stale results, schemas, stores, faithful fallback                                                   | 68 server and 6 Chromium tests            |
| Installed scalar package, capability selection, boundaries, repeated-use memory                                           | 17 tests across Chromium, Firefox, WebKit |
| Ordinary automatic policies and actual compositions                                                                       | 23 tests across Chromium and Firefox      |

Each scalar engine also verifies 367 frozen Wasm Yliluoma vectors and 734 untimed actual benchmark-adapter calls. Coverage retains the existing resize, matching, field, diffusion, Process, progress, durable-output, and failure fixtures. Passing these fixtures does not clear S41's different bilinear workloads.

Actual scalar memory is 1,179,648 bytes initially and 1,245,184 bytes after warmup and both repeated batches. The processing-capacity budget is 262,144 bytes. Maximum-axis checks reach 3,145,728 Wasm bytes under an 8,388,608-byte capacity budget. Disposal preserves outputs without shrinking Wasm pages. This checks bounded reuse and preflight, not successful maximum-area processing or browser heap limits.

Tools are Node 24.19.0, pnpm 11.13.0, TypeScript 6.0.3, Rust 1.97.0, threaded nightly-2024-08-02, wasm-pack 0.15.0, wasm-bindgen 0.2.121, and wasm-opt 117. Browsers are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 with Playwright 1.59.1. Full tool identities remain in retained build provenance.

The first local Chromium command names a nonexistent executable and exits before launch. The corrected path exposes the fixture-import failure described above. The completed three-engine run follows the repair; failed attempts are not counted as passes.

## Reused evidence and size

Full native scalar/threaded conformance, 424/425 tests, and 12 threaded lifecycle checks remain [S42 evidence](84-package-report.md). The source/build-input comparison against S42's actual build source `bdbcb3c812701d50f157a12d7f157138f10b4013` is empty for core source, Cargo files, toolchains, factory/build scripts, public source, website source, and frozen guard. The exact tarball matches. No duplicate native suite or threaded lifecycle run is claimed.

The [S41 performance reports](83-release-status.md) retain their original measured revisions and artifacts. Comparison with the continuation inventory confirms all 46 runtime package files match; only README and package metadata differ. Executable TypeScript files match too. The new oracle has its own identity and fresh target-local conformance. No timing is relabeled as a measurement of S43, and no performance run is repeated here.

| Artifact         | Raw bytes | Gzip bytes | Brotli bytes |
| ---------------- | --------: | ---------: | -----------: |
| Scalar Wasm      |   380,928 |    167,411 |      134,397 |
| Threaded Wasm    |   548,630 |    202,542 |      160,932 |
| Complete tarball | 1,190,912 |    407,888 |      277,574 |

Fresh per-file compression and installed/tarball comparisons equal S42. The checker finds no growth against S42's proposed baseline. That baseline still awaits review, including package-manifest growth above 10% against S40. Passing the size comparison does not approve publication.

## Required release blockers

| Gate                              | Evidence and action needed                                                                                                                                      |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Confirmed performance regressions | 18 cells: seven historical, nine TypeScript, two automatic Firefox warm controls. New exact optimization work needs bounded fresh comparisons.                  |
| Missing scalar release coverage   | Firefox cases 9–14 and WebKit 0–14, 21 cells. Interrupted Firefox 9 needs both fresh alternating pairs.                                                         |
| Maximum indexed output            | Playwright pipe transport fails its string bound. Repair transport and pass the untimed 8192×8192 feasibility check before capped measurements.                 |
| Frozen drift and noise            | Bilinear mismatches remain unapproved. Inconclusive anchor, scalar compiled-startup, candidate, and automatic warm controls cannot pass by pooled median alone. |
| Threaded WebKit                   | Demonstrate atomic-wait worker cleanup before certifying that engine. Scalar WebKit passes do not satisfy this gate.                                            |
| Publication and human decisions   | Initial size review, trusted npm publisher/protected environment setup, visual/stability acceptance, default activation, and retirement activation remain held. |

General website field/diffusion/perceptual/mixing equivalence also remains unavailable; TypeScript evidence covers admitted fixtures only. Frozen spec/image/guard and production kernels remain unchanged. The package path stays behind `import.meta.env.DEV` and `VITE_DITHERETTE_WASM_PROCESS === 'true'`. No publication, tag, deployment, merge, default activation, or retirement activation occurs here.

## Reproduction

From a clean checkout, install frozen dependencies and fetch the locked bench/oracle/syntax manifests. Use the pinned toolchains and Playwright engines. Fresh preparation and fixture generation are:

```sh
pnpm install --frozen-lockfile --offline --ignore-scripts
for manifest in crates/ditherette-bench/Cargo.toml crates/ditherette-wasm/Cargo.toml crates/ditherette-bench-oracle/Cargo.toml tools/spec-freeze/syntax/Cargo.toml; do cargo +1.97.0 fetch --locked --manifest-path "$manifest" || exit; done
mkdir -p target
node scripts/prepare-public-benchmark.mjs target/s43-public
cargo +1.97.0 run --locked --release --manifest-path crates/ditherette-bench/Cargo.toml --target-dir target/compiler --example yliluoma_conformance -- target/yliluoma-conformance.json
```

Set `DITHERETTE_TEST_TARBALL` and its SHA-256, `DITHERETTE_BENCH_ORACLE`, and `DITHERETTE_BENCH_YLILUOMA_FIXTURES` to these absolute paths. Run `pnpm --filter ditherette-wasm test:conformance`. Set `DITHERETTE_BENCH_AUTO_BUNDLE` to the prepared `bundle-source.json` for `test:conformance:automatic`. Optional engine executable variables select owned runtimes. [S40 commands](../docs/plans/ditherette-v1/s40-conformance.md#repeatable-commands-and-ci) and `.github/workflows/package-conformance.yml` define the unchanged package/web checks.

The fresh native command is `cargo +1.97.0 test --locked --release --manifest-path crates/ditherette-bench/Cargo.toml --target-dir target/compiler --lib --test browser_worker --test paired_browser --test paired_quantize --example release_integration_plan`.

For unresolved performance gates, generate a fixed case with `release_integration_plan LANE CASE_INDEX NEW_JSON HOST_LOAD_NOTES`. The [S41 reproduction instructions](83-release-status.md#reproduction-and-holds), [paired protocol](../crates/ditherette-bench/PAIRED.md), and [exclusive execution rules](../crates/ditherette-bench/EXECUTION.md) govern fresh artifacts and approved measurement windows. Missing coverage is a blocker, not authority to extend the experiment budget. `release.mjs verify` and `publish-check` retain S42's publication hold checks; no publisher credentials are inspected here.

After validation, all owned jobs exit and all finished target directories are removed. Source, committed reports, and the small artifact/provenance bundle remain. Root owns the final stack-audit join and unmerged PR.
