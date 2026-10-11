# Exact Yliluoma mixture index

Yliluoma tests every rounded palette pair and ratio. The production index stores those exact
candidates in their frozen enumeration order. Tree traversal may reject a subtree only through a
lower bound. The surviving candidate always uses the frozen policy score, and equal scores keep
the earliest candidate.

The first index supported Oklab only because that was the original optimization's scope. It did
not depend on an Oklab-specific property. Every Euclidean working space interpolates the same
three stored coordinates componentwise, so the same squared-distance tree applies unchanged.

Other policies use these exact bounds:

- Fixed weighted RGB scales a split-axis gap by that axis's metric weight.
- CompuPhase uses its global channel-weight minima over normalized sRGB.
- Circular hue indexes `(L, C cos h, C sin h)`. This is the cylindrical chord distance, but the
  frozen score still resolves every survivor.
- Hue arc uses the same polar index divided by the proven worst-case factor `1 + pi/2`, plus a
  periodic `L/C/h` subtree bound.
- CIEDE2000 stores subtree Lab boxes and candidate chroma. It seeds with the nearest Lab mixture,
  then applies coarse Lab, lightness, chroma/hue, and final exact bounds in order.

The optional RGB mixture cache scales with the image through the shared cache sizing policy.
Adaptive placement splits it into source and nearest-colour endpoint caches. Intermediate masks
remain position-dependent and use the index directly.

## Reuse across frames

The index depends only on the prepared palette and the Bayer level count. A processor keeps it in
its preparation store under the palette preparation identity plus the level count, so a new frame
with the same palette, alpha, metric, and matrix skips the rebuild. Placement and source bytes are
not part of the key. The index counts toward the store's byte cap and LRU eviction like any other
preparation, and only a successful call publishes a new one. Without spare capacity, the call keeps
the literal search. One-shot calls outside a processor still build a call-local index.

Building the 63-colour Wplace Oklab index takes about 1, 4, 17, and 73 ms natively for 2×2, 4×4,
8×8, and 16×16, and retains 0.3, 1.0, 3.7, and 14.5 MB.

## Benchmarks

Scalar Wasm in Node, 2026-09-28: medians of three `process` calls on a fresh processor, each on a
new 600×400 image of smooth ramps plus small per-pixel noise, with the 63-colour Wplace palette. The build used the release profile's existing `opt-level = "s"`.

The issue's 4×4 baseline, measured in Chromium, was 78 ms for Oklab, 4.7 s for CIELAB Euclidean, 9.1 s for
weighted RGB, 12.5 s for OKLCH, 13.4 s for CIELCh, and more than 60 s for CIEDE2000. This VM
measured the unchanged Oklab index at 133 ms. The final Oklab median is 136 ms, a 2.3% change.

### Everywhere placement after indexing

| Matching policy      |      2x2 |      4x4 |      8x8 |    16x16 |
| -------------------- | -------: | -------: | -------: | -------: |
| sRGB Euclidean       |   120 ms |   136 ms |   195 ms |   392 ms |
| Linear RGB Euclidean |   121 ms |   127 ms |   183 ms |   378 ms |
| Oklab Euclidean      |   129 ms |   136 ms |   200 ms |   407 ms |
| CIELAB Euclidean     |   127 ms |   138 ms |   200 ms |   418 ms |
| YCbCr Euclidean      |   123 ms |   137 ms |   189 ms |   400 ms |
| sRGB CompuPhase      |   127 ms |   146 ms |   212 ms |   447 ms |
| sRGB Rec.601         |   126 ms |   136 ms |   202 ms |   402 ms |
| sRGB Rec.709         |   130 ms |   137 ms |   201 ms |   406 ms |
| OKLCH Euclidean      |   127 ms |   169 ms |   307 ms |   742 ms |
| OKLCH circular hue   |   321 ms |   380 ms |   540 ms |   866 ms |
| OKLCH hue arc        |   296 ms |   382 ms |   552 ms |   895 ms |
| CIELAB CIEDE2000     | 1,126 ms | 1,193 ms | 2,285 ms | 6,461 ms |
| CIELCh Euclidean     |   128 ms |   149 ms |   214 ms |   447 ms |
| CIELCh circular hue  |   316 ms |   392 ms |   556 ms |   903 ms |
| CIELCh hue arc       |   299 ms |   381 ms |   529 ms |   841 ms |

CIEDE2000 is the only 4x4 exception to the 3x Oklab target. Its pair-dependent chroma scaling and
hue rotation make the safe bounding metric loose. Each surviving candidate also needs square
roots and trigonometric exact verification. The index still cuts the reported runtime by more than
50x without changing a tie.

### Adaptive placement after indexing

| Matching policy      |    2x2 |    4x4 |      8x8 |    16x16 |
| -------------------- | -----: | -----: | -------: | -------: |
| sRGB Euclidean       | 196 ms | 220 ms |   326 ms |   784 ms |
| Linear RGB Euclidean | 192 ms | 229 ms |   348 ms |   819 ms |
| Oklab Euclidean      | 206 ms | 247 ms |   382 ms |   959 ms |
| CIELAB Euclidean     | 209 ms | 239 ms |   344 ms |   752 ms |
| YCbCr Euclidean      | 195 ms | 226 ms |   341 ms |   863 ms |
| sRGB CompuPhase      | 227 ms | 251 ms |   364 ms |   807 ms |
| sRGB Rec.601         | 195 ms | 229 ms |   359 ms |   914 ms |
| sRGB Rec.709         | 199 ms | 225 ms |   370 ms |   940 ms |
| OKLCH Euclidean      | 220 ms | 255 ms |   367 ms |   776 ms |
| OKLCH circular hue   | 725 ms | 843 ms | 1,333 ms | 3,475 ms |
| OKLCH hue arc        | 538 ms | 638 ms | 1,054 ms | 2,758 ms |
| CIELAB CIEDE2000     | 748 ms | 858 ms | 1,212 ms | 2,488 ms |
| CIELCh Euclidean     | 241 ms | 262 ms |   368 ms |   754 ms |
| CIELCh circular hue  | 752 ms | 853 ms | 1,252 ms | 2,562 ms |
| CIELCh hue arc       | 629 ms | 665 ms |   952 ms | 2,038 ms |
