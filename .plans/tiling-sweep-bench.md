# Tiling sweep benchmark design

Goal: empirically fit smooth tiling-parameter curves for each production resize filter using the same real fixtures as scalar resize perf work:

- `Celeste_Insta_selfie`: 800x800
- `Celeste_box_art`: 2600x4168

Do not encode hand bands like “if pixels are between A and B use config C”. The sweep should collect measurements over actual source/output dimensions, then fit continuous functions that predict good tiling parameters.

## Scope

Filters:

- nearest
- area
- bilinear
- bicubic fixed
- bicubic scale-aware
- lanczos2 fixed
- lanczos2 scale-aware
- lanczos3 fixed
- lanczos3 scale-aware

Tiling primitives:

- row bands first: `prod::tiling::RowBandPlan`
- tile grids later only if row bands cannot model a filter well

The current `prod::tiling` module is geometry/execution only. A real filter sweep needs benchmark-only or production row-range adapters that compute `output_y_start..output_y_end` with absolute output coordinates so tiled and scalar outputs match.

## Benchmark shape

Add a separate bench command, not a manifest perf profile:

```sh
cargo run --release --manifest-path crates/ditherette-bench/Cargo.toml -- \
  tiling-sweep --fixtures Celeste_Insta_selfie,Celeste_box_art \
  --filters nearest,area,bilinear,bicubic,bicubic-scale-aware,lanczos2,lanczos2-scale-aware,lanczos3,lanczos3-scale-aware
```

Outputs:

- `crates/ditherette-bench/target/tiling-sweep/raw.csv`
- `crates/ditherette-bench/target/tiling-sweep/best.csv`
- `crates/ditherette-bench/target/tiling-sweep/curves.json`
- `crates/ditherette-bench/target/tiling-sweep/report.md`

## Dimension grid

Use actual dimensions, not abstract scale labels. For each fixture, generate output dimensions by preserving the fixture aspect ratio and rounding exactly like the existing resize harness.

For the two fixtures this creates concrete `source_w,source_h,out_w,out_h` rows. The sweep records dimensions directly; scale is only a derived feature.

Dimension sampling:

- dense near identity: output long edge at `0.90..1.10` source long edge in 1–2% steps
- common downscales: long edge targets around 64, 80, 100, 160, 200, 260, 325, 400, 520, 650, 800, 1042, 1300, 1950, 2340, 2470, 2574
- common upscales: 1.25x, 1.5x, 2x, 3x, 4x actual rounded dimensions
- include exact identity for scalar/tiled overhead boundary

Also include a small anisotropic slice later, but keep first pass uniform aspect-preserving so curves are learnable.

## Parameter sweep

For row bands, sweep a generated target band height rather than old fixed config bands:

- `target_band_height`: powers and midpoints from 8 rows to full output height
- derived features: band count, pixels per band, bytes per band
- executor mode: scalar baseline vs row-band tiled
- worker cap: measure 1, 2, 4, 6, 8 if/when parallel executor exists; current generic executor is sequential, so first implementation can collect partition overhead and row-range correctness only

For tile grids later:

- `target_tile_width`, `target_tile_height`
- constrain tile bytes to a plausible cache range, but do not hard-code final cache bands

## Measurement protocol

For each `(fixture, filter, output_dimensions, tiling_params)`:

1. Run scalar production resize once as oracle output.
2. Run tiled resize with absolute output coordinates.
3. Verify exact output for nearest; bounded output for other filters using existing benchmark bounds.
4. Measure warm throughput with the existing timing loop style.
5. Record median, mean, p95, stdev, throughput, checksum, and correctness deltas.

Use two baselines per case:

- scalar production time
- best measured tiled time

The optimization target is speedup over scalar with bounded p95/stdev constraints, not just fastest median.

## Curve fitting

Fit per-filter continuous response models from raw rows.

Features:

- `source_pixels = source_w * source_h`
- `output_pixels = out_w * out_h`
- `source_aspect`, `output_aspect`
- `scale_x = out_w / source_w`
- `scale_y = out_h / source_h`
- `minify_x = max(source_w / out_w, 1)`
- `minify_y = max(source_h / out_h, 1)`
- `kernel_cost`: nearest=1, bilinear=4, bicubic/lanczos2 fixed=16, lanczos3 fixed=36, scale-aware uses measured average tap product
- `target_band_height`
- `pixels_per_band`
- `band_count`
- `bytes_per_band`

Fit target:

```text
speedup = scalar_median / tiled_median
```

Model shape:

- fit `log(speedup)` with a smooth additive model or ridge-regularized spline basis
- predict over a dense candidate grid of `target_band_height`
- choose the smallest parameter within 1% of predicted best to reduce noise sensitivity
- enforce smoothness by penalizing high curvature in `log(output_pixels)`, `log(kernel_cost)`, and `log(bytes_per_band)`

Generated policy should be formula-based, e.g.:

```text
target_band_pixels(filter,dims) = exp(a0 + a1*log(output_pixels) + a2*log(kernel_cost) + ...)
target_band_height = clamp(round(target_band_pixels / out_w), min_rows, out_h)
```

No lookup tables of dimension ranges. If a discrete worker count is needed, choose it by evaluating the smooth model over worker candidates and picking the predicted best.

## Acceptance criteria

A fitted curve is acceptable for a filter when:

- both fixtures improve or stay within noise for representative dimensions
- p95 does not regress more than median speedup gains justify
- exact/bounded correctness passes for every tiled case
- the curve chosen from training cases also holds on a small holdout dimension set
- the generated policy is monotonic or near-monotonic where expected: larger output/kernel cost should not produce wildly smaller work units without measured evidence

## Implementation slices

1. Add row-range resize adapters for one cheap filter, probably nearest, behind benchmark-only plumbing.
2. Add `tiling-sweep` command that writes raw CSV for nearest only.
3. Add curve fitter script/command over CSV and validate holdout dimensions.
4. Expand adapters/filter matrix one filter at a time.
5. Only after empirical curves are stable, wire production row-band policy.
