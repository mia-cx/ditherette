# S23 and S24 validation join

Validate coordinator merge `d14b166f4171b028c8b194a533b16cfdccf7b10c` without measurements.
Its parents are delivered trilinear integration `b9b7f96b` and S24 `884a8868f52e82ae579835eee173c6385a9c9ca2`.
The coordinator owns progress and existing PRs. This task owns validation evidence and any necessary focused merge corrections only.

## TODOs

- [x] Review shared merge resolutions and verify both kernel families retain their delivered bytes.
- [x] Run native implementation, benchmark protocol/adapter, formatting, Wasm, and trusted freeze checks.
- [x] Build both package variants and validate interface/types, private ABI, and installed tarball in three engines.
- [x] Check the actual benchmark adapter against the new tarball without timers; record evidence and drain validation sessions.

Reuse this coordinator's existing Wasm target and the exclusive S24-bench benchmark target.
Initial free space was 3.4 GiB. Compilation reduced it to 1.9 GiB.
The authorized old S23-trilinear dev-cache cleanup removed 5,976 files totaling 1.9 GiB.
Its release variants, source, and immutable artifacts remain untouched.
Preserve every immutable S23/S24 artifact and result directory. No benchmark process or measurement is authorized.

## Merge review and correction

The joined protocol retains trilinear and indexed quantize requests, identities, registrations, and explicit TypeScript exclusions.
The public page retains S24's every-output observer, including indexed metadata and alias checks.
Its only functional change against S24 adds the trilinear recipe and TypeScript exclusion.
The merged conformance suite retains 17 frozen quantize fixtures and adds three odd-mip trilinear anchor vectors.

Both source comparisons returned no differences:

```sh
git diff --exit-code d14b166f^1 d14b166f -- crates/ditherette-wasm/src/prod/resize
git diff --exit-code 884a8868f52e82ae579835eee173c6385a9c9ca2 d14b166f -- crates/ditherette-wasm/src/prod/quantize crates/ditherette-wasm/src/prod/color.rs crates/ditherette-wasm/src/prod/color crates/ditherette-wasm/src/prod/palette
```

The first benchmark test run exposed one stale assertion in `tests/verification_adapters.rs`.
It expected trilinear production to remain unavailable after S23 had registered it.
The replacement compares the joined adapter with frozen output for a 3×1 odd-mip image.
Left, center, and right anchors produce red bytes 68, 85, and 103 respectively.
No production, reference, transport, or timing implementation changed during validation.

## Commands and results

Commands run from this worktree unless stated otherwise.
`BENCH_TARGET` below denotes `/home/mia/mia-cx/ditherette/.worktrees/v1-s24-bench/crates/ditherette-bench/target`.
`WEBKIT` denotes `/tmp/ditherette-webkit-libs.2dS6Yu/webkit`.

| Check | Command | Result |
| --- | --- | --- |
| Native core | `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --quiet` | 315 tests pass |
| Benchmark protocol and adapters | `CARGO_TARGET_DIR="$BENCH_TARGET" cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --tests --example quantize_integration_plan --example resize_integration_plan` | 59 tests pass after the stale assertion correction; one subprocess-only helper stays ignored by the outer runner |
| Controlled browser protocol | `node --test scripts/benchmark-public-timing.test.mjs scripts/benchmark-public-browser.test.mjs` | 26 tests pass; fake clocks only |
| Core formatting | `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check` | Pass |
| Benchmark formatting | `cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml -- --check` | Pass |
| Wasm compilation | `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --target wasm32-unknown-unknown` | Pass |
| Package builds | `pnpm package:build` | Scalar, threads, factory staging, and declarations pass |
| Public interface and types | `pnpm --filter ditherette test:interface` | Type checks and 22 tests pass |
| Private ABI | `node --test crates/ditherette-wasm/tests/private_processor.mjs crates/ditherette-wasm/tests/private_quantize.mjs` | 10 tests pass |
| Installed package | `DITHERETTE_TEST_WEBKIT_EXECUTABLE="$WEBKIT" pnpm --filter ditherette test:browser` | Chromium, Firefox, and WebKit pass; four test records including the parent |
| Bench compilation only | `CARGO_TARGET_DIR="$BENCH_TARGET" cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --benches` | Pass; no measurement |
| Trusted freeze | `node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-resize-integration --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze` | Pass, including isolated compilation |

The freeze guard reports revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and unchanged content identity
`sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Package builds retain the existing unstable-atomics and optional wasm-pack package-metadata warnings.

The scoped cleanup command was:

```sh
cargo clean --profile dev --manifest-path /home/mia/mia-cx/ditherette/.worktrees/v1-s23-trilinear/crates/ditherette-wasm/Cargo.toml
```

## Fresh tarball adapter conformance

The package contains the joined implementation from `d14b166f`.
The build occurred at plan-only child `8c9694b60095451dbf84a4221056a71339ddddf1`, with the test correction uncommitted.
Those plan and test changes do not affect package bytes.
This validation tarball is not a prepared measurement artifact and has no performance claim.

```sh
pnpm --dir packages/ditherette pack --out /home/mia/mia-cx/ditherette/.worktrees/v1-resize-integration/target/s23-s24-validation-d14b166f/ditherette.tgz
DITHERETTE_BENCH_TEST_TARBALL=/home/mia/mia-cx/ditherette/.worktrees/v1-resize-integration/target/s23-s24-validation-d14b166f/ditherette.tgz \
DITHERETTE_BENCH_QUANTIZE_FIXTURES=/home/mia/mia-cx/ditherette/.worktrees/v1-s24-bench/crates/ditherette-bench/target/s24-quantize-attempt-02/conformance.json \
DITHERETTE_TEST_WEBKIT_EXECUTABLE=/tmp/ditherette-webkit-libs.2dS6Yu/webkit \
node --test scripts/benchmark-public-conformance.test.mjs
```

All three engines pass actual adapter conformance without entering the measurement collector.
This covers trilinear odd-mip anchors, 17 frozen quantize fixtures in primed and fresh instances, existing resize recipes, and honest TypeScript exclusions.
Engine versions are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
The suite reports four passing test records including the parent.

| Local artifact | SHA-256 |
| --- | --- |
| `target/s23-s24-validation-d14b166f/ditherette.tgz` | `4dea3ff36ab515912cac1c30f01b827c4f65041d01bb4e74218d6720cd6e54c6` |
| `packages/ditherette/dist/wasm/scalar/ditherette_wasm_bg.wasm` | `34c9c2353aa9b71b7d8b5995e9d318d75b6a1f98d84b30cc10a4d6026d057758` |
| `packages/ditherette/dist/wasm/threads/ditherette_wasm_bg.wasm` | `cc5cb5cf7663688d9795b9f9198bbb0eb5c6a1ae493b47fe0eee1690a20ae901` |

All validation sessions exited. No benchmark trial, promotion, publication, or PR change occurred.
The coordinator owns the subsequent integration and measurement decisions.
