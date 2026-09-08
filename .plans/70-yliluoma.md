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

Literal math baseline is `9718b168`. The request adapter follows the frozen pixel loop, including original-source adaptive placement after alpha-prepared target conversion.
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

## Next bounded scope

Add the Yliluoma branch to bounded Processor execution and private/public tagged dispatch, using this literal search unchanged.
Extend actual installed-package conformance and public-call benchmark registration before measuring.
After that baseline exists, one candidate can reuse bounded call-owned conversion or mixture preparation across pixels.
Any prepared mixture storage needs an explicit capacity limit and a no-table fallback for large palettes.
Keep this exhaustive implementation when an experiment loses or exactness/public gates remain incomplete.
