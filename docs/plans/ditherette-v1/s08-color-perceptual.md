# S08 perceptual color round trips

Implements [issue 49](https://github.com/mia-cx/ditherette/issues/49).
Branch: `impl/v1-s08-color-perceptual`. PR base: `impl/v1-s03-contracts`.
Validated dependency: `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`, [S03 PR 89](https://github.com/mia-cx/ditherette/pull/89).

## Work

- [x] Complete cartesian Oklab and D65 CIELAB pixel/image round trips with known vectors.
- [x] Complete cylindrical conversions, neutral/hue conventions, domains, and strided alpha-preserving fixtures.

## Decisions and evidence

Retain the inherited f32 forward matrices and D65 white. Formula sources and numerical conventions live beside each color module.
S07 proceeds independently in its own modules; this branch has no S07 dependency or shared-helper edits.
No production code or benchmark measurements belong to this slice.

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_color_perceptual --test spec_color_spaces` passes 16 focused tests.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --quiet` passes all 124 native tests and zero doctests.
`cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown` passes.
`cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` and `git diff --check` pass.

All four modules export `rgb8_to_<space>`, `<space>_to_rgb8`, the existing forward image adapter, and the corresponding inverse image adapter.
Image inverse arguments are a triplet image, the corresponding RGBA8 alpha source, and mutable RGBA8 output.
Exact gray inputs have C=0 and h=0 in cylindrical form. Near-grays retain their chroma and hue.
Documented domain boxes are conservative enclosures; S12 still owns selection and derivation of fixed placement normalization ranges.

Sources checked on 2026-09-07: [Ottosson's published Oklab matrices](https://bottosson.github.io/posts/oklab/) and [W3C conversion equations](https://www.w3.org/TR/css-color-4/#color-conversion-code).
Primary vectors were independently evaluated from those formulas in f64 and compared against f32 output.
CIELAB retains the inherited D65 white and XYZ matrix; its inverse matrix was calculated from that existing decimal matrix.
