# S24 prepared attempt 02

## Status and identities

Prepared and unmeasured. The coordinator owns quiet clearance and all 124 possible serial workers.
Attempt 01 remains unchanged. Its public snapshots lack retained-batch stability verification and must not provide final S24 evidence.

Paths below are relative to `.worktrees/v1-s24-bench/crates/ditherette-bench/target/s24-quantize-attempt-02/`.
Fresh worker, coordinator, and public build source is `f4b90ecfcde63531fb992cf87ebda04d4e373032`.
Public accepted/candidate roles use this identical package-capable artifact as explicitly labeled controls, not optimization evidence.
The newly built tarball matches attempt 01 because package code did not change. Transport bytes and provenance changed.

Native accepted source remains `ccb9bceb28563c562dd5e6c05f68c056c18e3519`, a thin protocol overlay on literal `a23260edec0452fd17c13073636f548b07804230`.
Both retain production tree `d4847ee149dfa346a85a23f8be28e6c9eb1fb768`.
The accepted binary is reused from `../s24-quantize/native-accepted/ditherette-bench` and copied into the new prepared pair.
Native request types, identity, timer, and worker source compare byte-identical between accepted overlay and the new source.
No old samples are reused. The candidate still calls current prepared quantize through the full-call convenience path.

## Retention and comparison scope

Public observers check every warmup and measured result after its call or full batch timer.
They retain the first actual and first distinct actual outputs, including A/B/A changes to indices, palette, transparency, or warnings.
They reject reused result records, metadata records, arrays, and backing buffers within a batch or retained evidence.
The public subject contract requires independent records and durable owned buffers after return.
The 64 MiB bound covers declared result buffers plus conservative record allowances, not the total JavaScript heap.
Indexed reservation uses one byte per pixel, 1,024 palette bytes, and 1,024 bookkeeping bytes per result.
Oversized calibrated batches fail before timing; calibration and batch size are never silently reduced.
Retention changes output lifetimes and GC behavior. It is part of this new public timing scope.

Native full-call timing still includes destruction. It checks only before/after outputs for fixed deterministic registered callables.
It does not certify arbitrary stateful subjects or detect transient native A/B/A behavior.
Packed-forward controls retain their identical per-iteration optimizer barrier and caller-owned output scope.
No application cache, benchmark-only public hash, or faithful TypeScript quantize adapter is claimed.

## Files and SHA-256

| File | SHA-256 |
| --- | --- |
| `../s24-quantize/native-accepted/ditherette-bench` | `3a888084f2c9f9bea610eef4909100492816111034b99b826fd6bc4e6a316916` |
| `native-candidate/ditherette-bench` | `2e6ce05e739cee84bc067b241800ade7197c833fff15b63bef5582dea2f54d3c` |
| `ditherette-bench-pair` | `b93d13d1552e7340b58a200667daff6af670f73a1bb9bdf789664c032c258687` |
| `public-build/ditherette.tgz` | `39f607810adce816973ad4adb0301607281698622341cd72e6edc02fcb0d3eee` |
| `public-build/build-provenance.json` | `b4777d04ea346c53cc015a37cdd64edc1e10ac07ce427ffefc01f62b6d4c77c0` |
| `conformance.json` | `ea98f1b8379afc434cbeae19d3822eab36b83d7d1fa9e1351559935d20c3dd60` |
| `prepared-native/prepared.json` | `045e4791031cbe527b99c52fb63a7a63c216094742ab725e9135cd3938458979` |
| `prepared-chromium/pair/prepared.json` | `e994b40f8e4e23701befe98584ab604599bac1275995117f4b2ca95b7cfe9742` |
| `prepared-firefox/pair/prepared.json` | `f95198cc4af9f98a53c0eb1ffb8ce1a05ea0d6f5d7215e1cdef1ce93962f7f99` |
| `prepared-webkit/pair/prepared.json` | `b741f986d7a0a71630a8e8e5af7912a7155d09ee454b3824a7d412d9c5684323` |

Each browser snapshot contains complete package, TypeScript, transport, browser, and Playwright closures plus source/build provenance.
Sources reuse the validated S23 runtime descriptions, including the relocated alias-preserving WebKit install.
Node is v24.19.0; Playwright is 1.59.1. Chromium is 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
All prepared engines are headless and isolated with no extra launch arguments.
The prepared pairs bind their executable bytes, complete served assets, runtime closures, and exact revisions.
Snapshots do not require a mutable source checkout. Rebuilding requires the recorded source revision.

## Validation and budget

The fixed plan has 13 native cases and six public cases per engine, each with two AB/BA pairs.
That permits 52 native and 24 workers per engine, totaling 124 serial workers.
Settings remain 128×96 fixtures, 20 samples, 50 ms warmup, a 250 ms cap, and a 2 ms throughput target.
These preparation notes do not attest that the host is quiet.

The following checks pass:

```sh
cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --tests --example quantize_integration_plan
node --test scripts/benchmark-public-timing.test.mjs scripts/benchmark-public-browser.test.mjs
cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --target wasm32-unknown-unknown
cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml -- --check
cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check
node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s24-bench --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze
```

Totals are 56 Rust tests and 26 controlled JavaScript tests.
One subprocess fixture is ignored in its parent suite, then invoked twice. Three Node cleanup fixtures also pass.
The actual installed-tarball adapter passes all three engines, four Node tests including the parent suite.
It checks 17 frozen quantize fixtures with primed/fresh preparation and the same retained-output observer, without measurement timers.
Frozen revision remains `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
Frozen digest remains `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Both native release binaries and scalar/threaded package builds pass using existing targets.
The first public preparer invocation found no parent attempt directory and exited before building.
Creating that scoped directory and rerunning succeeds. No artifact was overwritten or removed.
The existing threaded atomics warning and wasm-pack optional metadata warnings remain unchanged.
All artifact preparation and untimed browser sessions exit successfully. No measurement commands have run.
