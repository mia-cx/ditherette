# Blue-noise reference

## Pre-freeze correction

The inherited `BLUE_NOISE_8X8` was exactly the transpose of `ordered::bayer_value(x,y,8)`.
At half occupancy its checkerboard concentrates all non-DC Fourier power in one frequency.
It was an ordered Bayer tile under a misleading name, not the intended blue-noise field.
The old bytes remain recoverable in Git; the corrected reference uses a 32x32 rank tile.

## Source and construction

[Ulichney's 1993 void-and-cluster paper](https://cv.ulichney.com/papers/1993-void-cluster.pdf) supplies the ranking algorithm.
Relax a seeded binary pattern by moving its tightest occupied cluster to its largest void until that move restores the same site.
Assign low ranks by removing occupied clusters. Restore the relaxed pattern, then fill voids to half occupancy.
Finish by removing clusters of the now-minority empty sites. Every position receives a distinct rank.

The periodic Gaussian uses sigma 1.5 pixels. This choice follows the range investigated in
[Ulichney's filter study](https://cv.ulichney.com/papers/1994-filter-design.pdf).
Periodic distance uses the shortest displacement on each axis, counting every tile site once.

`blue_noise/generator.rs` is the naive offline executable reference, with direct density and Fourier sums.
`examples/generate_blue_noise.rs` only prints its construction result and fails if quality gates fail.
Neither runs during package initialization. Void-and-cluster is not a new public dither mode.
Seed, shuffle, phase boundaries, tie order, and preregistered numerical limits live in
[the S14 construction record](../../../../../docs/plans/ditherette-v1/s14-blue-noise.md).

## Rank field

`blue_noise/tile.rs` stores row-major unsigned ranks 0..1023. Its canonical digest input is each rank encoded as little-endian u16.
SHA-256: `bcd93746b99ef8ad678ad425f21e1890b4248050b1ea1b382800d7da977e5943`.
`blue_noise/analysis.json` records construction parameters and every specified spectral measurement.
The original parameters passed on the first generation; no seed search or relaxed thresholds were used.

`blue_noise_at(x,y)` returns `(rank + 0.5)/1024 - 0.5` at `(x mod32,y mod32)`.
All samples are exactly representable f32 dyadic fractions. Their tile mean is exactly zero; neither endpoint is sampled.
Coordinates always address the complete source image, including when output rows are processed separately.
The lookup reads no palette and supplies one centered scalar per pixel.

The inherited indexed-coordinate adapters add `strength*noise` to each f32 channel before their caller-selected nearest matcher.
The complete RGBA8 perturbation recipe additionally applies fixed working-space ranges, the quarter-range field scale,
and the palette-independent placement mask before inverse conversion. It preserves original byte alpha, including hidden RGB.
S13 owns this shared composition; S17 joins its field dispatch to this lookup before the reference freeze.

## Verification and reproduction

Run `cargo run --manifest-path crates/ditherette-wasm/Cargo.toml --locked --release --example generate_blue_noise`.
The generator uses f64 scalar arithmetic on the recorded Rust/Linux toolchain; the retained integer asset is authoritative for all runtimes.
Cross-toolchain regeneration must compare the complete rank array, not just aggregate scores.
`tests/spec_blue_noise.rs` checks regeneration, midpoint statistics, global coordinates, periodic density, negative spectral controls,
and independently calculated RGBA8 output with preserved alpha. Construction checks are not performance benchmarks or subjective-loss approval.
