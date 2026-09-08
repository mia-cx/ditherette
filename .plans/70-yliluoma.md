# S29 literal Yliluoma checkpoint

Issue #70. Branch `impl/v1-s29-yliluoma` starts at accepted S26 `3915f60519995cb9087a18b3bfd6bd7220ae804a`.
S27 remains untouched. The unselected S26 converter candidate is outside this ancestry.

## Scope and reuse

Copy missing exhaustive pair/ratio search, componentwise target adaptation, and strict global Bayer selection from frozen `spec/dither/yiluoma.rs`.
Reuse production Bayer ranks, placement, packed conversion, all fifteen metrics, palette/alpha preparation, and PreparedQuantizer.
Preserve enumeration order, f32 operations, original palette indices, zero-mask mixture search, and transparent-pixel bypass.
The native adapter borrows source storage and uses existing fallible capacity accounting. Mixture search allocates no cross-product table.

Read approved PRD/S29 and decisions #35, #37, #40 before implementation.
The frozen recipe remains the oracle; generated code changes only production and benchmark registration.

## TODOs

- [x] Copy missing literal math and verify all metrics, ratios, ties, target endpoints, and global thresholds; commit baseline.
- [x] Adapt the frozen scalar request loop to existing bounded palette preparation; verify complete output, alpha, placement, and budgets.
- [x] Register typed native benchmark adapters and verify oracle equivalence; commit and report the next bounded scope.

No optimization, measurements, public Processor/Wasm/TypeScript dispatch, or PR/issue writes in this checkpoint.
Native tests use exclusive `CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s22-convolution/crates/ditherette-wasm/target`.
Stop after the verified literal implementation and benchmark adapters. Public integration and measured optimization need their next assignment.

## Literal math validation

The production fragments are literal copies with imports redirected to existing production matching and Bayer helpers.
`cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_yiluoma` passes four tests.
Coverage includes 360 target/metric/matrix searches, original palette index gaps and duplicates, all 344 supported ratios over 1,156 global coordinate pairs, endpoint bits, hue interpolation, and explicit zero-mask/ratio ties.
No request adapter, prepared mixture table, memo, optimization, or benchmark execution exists in this baseline.

## Native request checkpoint

Literal math baseline is `9718b16841296c9094e304fe1c4b9d2b6f0e3b85`. The request adapter follows the frozen pixel loop, including original-source adaptive placement after alpha-prepared target conversion.
It reuses PreparedQuantizer through one crate-private `matcher()` borrow. Source conversion still constructs the existing converter per read.
Existing Budget reserves output fallibly and counts prepared capacities, the index record, and one temporary converter.
Mixture search retains constant scratch and no palette cross-product allocation. Source bytes remain borrowed.
Seven Yliluoma tests pass, including 540 full request combinations over all fifteen metrics, all matrix sizes, three alpha modes, and three placements.
Transparent-only, single-entry, 257-entry truncation, exact/one-under budgets, and validation precedence pass.
The six existing `prod_quantize` tests also pass after the shared accessor addition.

## Typed benchmark checkpoint

`prod:dither-and-quantize:yliluoma:literal-v1` uses the existing frozen `spec:dither-and-quantize:request:v1` oracle.
The native paired worker accepts typed `YliluomaSettings` and calls the actual bounded native routine.
Request mapping and verification serialization happen before samples. Each invocation includes validation, preparation, output allocation, and result destruction.
This scope borrows source bytes. It is not the future public call with JS/Wasm boundary copies.
Normalized settings bind palette order, alpha, matching, matrix size, and every placement control. Wrong families and invalid source/settings fail before invocation.

Two untimed adapter tests pass, including 45 metric/palette combinations using 2-, 16-, and 256-entry palettes.
`cargo check --locked --manifest-path crates/ditherette-bench/Cargo.toml --all-targets` passes.
Two older fixed-scope benchmark plan examples gain explicit unreachable Yliluoma arms for the extended native-operation enum.
No benchmark collector or measurement process runs.

## Delivered dependency join

Native request checkpoint is `3ef8cdc534072ce4b1fbfc526a4ca3c9c1999fe8`; typed adapter checkpoint is `5e82f83f991759f24b6362464ca83ef2b7ee174b`.
The branch joins S26 PR #115 head `bb36452ca831bad485f924e7ee007f9bbdb1cb0d`, based on S25 delivery.
That delivered tree equals the original accepted parent plus measurement records and the field-verifier repair.
Conflicts from rewritten S26 history retain the S29 additions and the delivered S26 records, verifier, and focused verifier tests.
The join's diff against delivered S26 contains only this S29 scope.
The S26 converter candidate remains absent from this branch.

After resolving the join, all benchmark targets typecheck and 19 focused benchmark/verification tests pass.
The inherited retained-evidence replay stays ignored; it requires the coordinator's immutable trial files and does not run here.
Both crate format checks pass. Frozen content verification retains revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and SHA-256 `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Frozen spec, shared image storage, and freeze policy have no diff from delivered S26.

## Next bounded scope

Add the Yliluoma branch to bounded Processor execution and private/public tagged dispatch, using this literal search unchanged.
Extend actual installed-package conformance and public-call benchmark registration before measuring.
After that baseline exists, one candidate can reuse bounded call-owned conversion or mixture preparation across pixels.
Any prepared mixture storage needs an explicit capacity limit and a no-table fallback for large palettes.
Keep this exhaustive implementation when an experiment loses or exactness/public gates remain incomplete.

## Authorized public literal baseline

The coordinator extends S29 from clean `65475193487b9b05a403525e6f36ae25d04a3a21` through public baseline validation.
Keep literal checkpoints retained; add no optimization or measurements.

- [x] Extract the unchanged pixel loop and wire bounded Processor execution; verify exact output and allocation boundaries.
- [x] Add borrowed private/public tagged Yliluoma dispatch and strict controls; build scalar/threaded artifacts and verify ABI/types.
- [x] Extend actual public benchmark adapters and installed-package fixtures; verify Chromium, Firefox, WebKit and report clean head.

Family 3 is Yliluoma. Reuse private field slots with matrix width in parameter and all unused controls zero.
S28 reserves dither error paths 26–35; Yliluoma matrix size uses 36. Shared placement tags match S28.

The allocation-free caller-owned loop preserves the literal math and still uses per-read conversion.
Processor reuses the existing quantize preflight/caught boundary, with exactly two buffer reservations and no RGBA8 intermediate.
Mode overhead counts its controls and the existing temporary converter. Separable capacity charges stay unchanged.
Sixteen focused native tests pass, including 180 full Processor combinations, exact/one-under budgets, both reservation failures, caught copy/completion recovery, and invalid placement before allocation.

## Public dispatch and target-local exactness

Processor checkpoint is `114d9e53`. Both scalar and threaded release builds pass with the unchanged literal pixel search.
The private ABI keeps its existing arity and borrowed/caught helpers. All 12 private tests pass.
Public tags expose all accepted matrix and placement controls, with strict unknown-field and f64-to-f32 checks.
All 25 interface/type tests pass. Exact budget, one-under before copy, output ownership, reentry, and caught failures pass.
Native coverage now includes all 367 public requests; 17 focused native tests pass.

The 367-case native fixture remains `packages/ditherette/tests/fixtures/yiluoma.json`.
SHA-256 is `5ecd3b20bfde622d76c1534ef3cb5b91be10ec42f2763b6aa00cf45497e455d7`.
The public fixture is `yiluoma-wasm.json`, generated independently by unchanged S18 frozen spec/image compiled to Wasm.
Its SHA-256 is `6dfd78cf6cc063bb53744003168160261f7204f4802c0ffbb785a04e825b50ca`.
The proof crate remains `/tmp/ditherette-s29-oracle.37ehus`; its complete frozen-only source is retained in `70-wasm-oracle-probe.rs`.
It uses serde 1, serde_json 1, sha2 0.10, wasm-bindgen 0.2.121, and release opt-level `s`.
Generated oracle Wasm SHA-256 is `ddc39f83f778eb4556fd71df858871f53ffff8fed85b3f066f821f564e9df663`.
No production code participates in fixture generation. The coordinator is adding the reusable frozen-only benchmark oracle separately.

Seven target differences occur at zero-based cases 236, 248, 251, 254, 257, 260, and 263.
All use matte alpha and CIELAB or CIEDE2000 matching with a gray palette.
For gray64, frozen native CIELAB a/b are zero; frozen Wasm gives approximately -0.000014901161 and 0.0000059604645.
Both production and spec share those coordinates within each target. Exhaustive mixture ties expose inherited target math rounding.
The minimized size4/CIEDE2000 case returns native `[2,2,1,1]` and frozen/public Wasm `[2,2,1,2]`.
The coordinator explicitly accepts exact target-local conformance, not universal cross-target parity.
Tests retain and check the seven differences. No tolerance, arithmetic repair, or spec change hides them.
Temporary production probe exports were removed before rebuilding both release variants.

## Public benchmark and installed-package checkpoint

Public implementation checkpoint is `158f76d4`. The typed public operation is `yliluoma` with `YliluomaSettings`.
Its subject is `public:dither-and-quantize:yliluoma:package`; the reference remains `spec:dither-and-quantize:request:v1`.
Native and public operations share the full normalized settings/input identity.
The real browser adapter constructs the accepted public request and calls `instance.ditherAndQuantize` for primed and fresh instances.
The unavailable TypeScript implementation rejects explicitly. No substitute algorithm or collector runs.
All benchmark targets typecheck and both focused adapter tests pass, retaining 45 metric/palette-size combinations.

An installed tarball passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each engine verifies 367 frozen Wasm Yliluoma vectors plus 734 untimed calls through the actual benchmark adapter.
Coverage includes all 15 matching policies, all four matrix sizes, all alpha policies, both placements, odd dimensions, transparent-only/single-visible/truncated palettes, and the explicit zero-mask tie.
Each engine also checks strict controls, source/result ownership, exact/one-under budgets, caught copy recovery, reentry, and disposal.
The existing 91 field vectors, 1,365 compositions, and prior installed-package checks still pass in each engine.
The retained WebKit launcher is `/home/mia/mia-cx/ditherette/.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit`.

Final scalar Wasm is 241,988 bytes, SHA-256 `3377430ae4369a5b22b24ee04235f2eab700ffcb0c1a9555ba51e7bdc10d8f25`.
Threaded Wasm is 331,861 bytes, SHA-256 `122cbbc9e3cc2f643f5a0ea76f9d8d7943a675239a137ee5123ad4ccf8cd39d6`.
Frozen content verification still reports the S18 revision and digest above. Spec, image storage, and freeze policy remain unchanged.

The complete literal public baseline is ready for coordinator measurement preparation.
Stop here. A next candidate may reuse the existing prepared converter per call, then consider bounded mixture preparation only after measurements.
Keep every literal checkpoint and compare against exact target-local frozen output. This checkpoint runs no measurements and opens no PR.
