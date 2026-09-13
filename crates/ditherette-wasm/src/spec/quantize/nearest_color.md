# nearest-color quantization

Nearest-color quantization maps each source pixel to a palette index.

The spec accepts precomputed color-coordinate buffers and matching palette coordinates. Color conversion and palette coordinate memoization live outside this module.

Tie rule: stable palette order wins. The nearest scan only updates the best candidate on strictly smaller distance, so exact ties keep the earliest palette entry.

## Complete RGBA8 quantization

`spec::quantize::quantize(QuantizeRequest)` composes request validation, palette preparation, byte-alpha handling, conversion, and exhaustive matching.
It returns `Result<IndexedImage, DitheretteError>` with durable palette and warning metadata.
All version, numeric, palette, and image checks finish before palette preparation or output allocation.
Raw metric tags first pass `parse_match` or recipe decoding. `MatchPolicy` then admits only the 15 coherent combinations.

The reference reads each source RGBA8 pixel in row-major order.
`PreparedPalette::prepare_pixel` either supplies a fixed index or RGB bytes for conversion.
Fixed indices bypass conversion and matching, including transparent-only output and preserve-mode darkest fallback.
Visible RGB enters `rgb8_to_coordinates` in the matching policy's space, then `PaletteMatcher::nearest`.
The result always uses original retained palette indices, never visible-entry ordinals.

`PaletteMatcher::new(&PreparedPalette, MatchPolicy)` converts every visible entry once, retaining duplicates.
Its public ordered `colors` contain `PaletteColor { index, coordinates }` for diffusion and palette-mixing reference compositions.
The matcher scans every entry and replaces the winner only when `distance_score` is strictly smaller.
It uses no early exit, pruning, spatial tree, quantization table, or memoization.
Transparent-only palettes have an empty matcher and must bypass `nearest`.
Internal callers supply finite coordinates in the matcher's working space.

`distance_score` returns squared distance except for CIEDE2000, which returns delta E.
The [metric reference](metric.md) defines each formula, its domain, and source evidence.
The [palette reference](../palette/README.md) owns exact alpha rounding and warnings.
`tests/spec_quantize_request.rs` checks complete-call outputs for every pair, stable ties, ownership, and boundary failures.
