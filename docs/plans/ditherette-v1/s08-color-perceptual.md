# S08 perceptual color round trips

Implements [issue 49](https://github.com/mia-cx/ditherette/issues/49).
Branch: `impl/v1-s08-color-perceptual`. PR base: `impl/v1-s03-contracts`.
Validated dependency: `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`, [S03 PR 89](https://github.com/mia-cx/ditherette/pull/89).

## Work

- [x] Complete cartesian Oklab and D65 CIELAB pixel/image round trips with known vectors.
- [ ] Complete cylindrical conversions, neutral/hue conventions, domains, and strided alpha-preserving fixtures.

## Decisions and evidence

Retain the inherited f32 forward matrices and D65 white. Formula sources and numerical conventions live beside each color module.
S07 proceeds independently in its own modules; this branch has no S07 dependency or shared-helper edits.
No production code or benchmark measurements belong to this slice.

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_color_perceptual --test spec_color_spaces` passes 11 tests after the cartesian step.
