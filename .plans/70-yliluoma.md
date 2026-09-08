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
- [ ] Adapt the frozen scalar request loop to existing bounded palette preparation; verify complete output, alpha, placement, and budgets.
- [ ] Register typed native benchmark adapters and verify oracle equivalence; commit and report the next bounded scope.

No optimization, measurements, public Processor/Wasm/TypeScript dispatch, or PR/issue writes in this checkpoint.
Native tests use exclusive `CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s22-convolution/crates/ditherette-wasm/target`.
Stop after the verified literal implementation and benchmark adapters. Public integration and measured optimization need their next assignment.

## Literal math validation

The production fragments are literal copies with imports redirected to existing production matching and Bayer helpers.
`cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_yiluoma` passes four tests.
Coverage includes 360 target/metric/matrix searches, original palette index gaps and duplicates, all 344 supported ratios over 1,156 global coordinate pairs, endpoint bits, hue interpolation, and explicit zero-mask/ratio ties.
No request adapter, prepared mixture table, memo, optimization, or benchmark execution exists in this baseline.
