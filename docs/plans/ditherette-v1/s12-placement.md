# S12 palette-independent placement

Implements [issue 53](https://github.com/mia-cx/ditherette/issues/53).
Branch: `impl/v1-s12-placement`. PR base: `impl/v1-s12-base`.
Validated join: `bdf79a606c94e8b44ee2c6fa11ea97af490b4b2e`.
Dependencies: S07 `7ef52bd2bcaea2774a400875e5c395526bc9b4b9` and S08 `d5e2d9761481f7a6b74fb37c7c2f7841570bead1`.
Both dependency commits are ancestors. The joined basic/perceptual/existing color suites compile and pass all 26 tests.

## Work

- [x] Derive fixed coordinate domains for all seven spaces and verify sampled gamut boundaries.
- [ ] Implement the eight-neighbor contrast and placement mask with edge, radius, threshold, and hue fixtures.

## Ownership

This slice changes only the reference placement module, its barrel, focused tests, and scoped documentation.
It accepts no palette and performs no palette preparation, color rewrite, production dispatch, or benchmark measurement.

## Evidence

The fixed-domain fixture passes for all seven spaces, including cube corners and transfer-branch byte boundaries.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_dither_placement` passes its first test.
YCbCr chroma bounds include a documented 0.000001 f32 allowance; the approved conversion remains unchanged.
