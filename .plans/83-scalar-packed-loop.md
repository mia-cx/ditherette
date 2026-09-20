# S41 packed color loop candidate

Base `ad755f85`, branch `perf/v1-scalar-packed-loop`.
Selection belongs to corrective issue #136 under #83.
The coordinator selects this candidate for native scalar production after the measurements below.

## Evidence and scope

The existing 101-case spec/production report records `forward-srgb` medians of 187,018/920,295.5 ns.
`forward-ycbcr` medians are 245,094/963,716.5 ns for the same 512x384 image size.
Report source is `.worktrees/v1-scalar-spec-bench/target/scalar-comparison/spec-prod-baseline-results/report.json`.

Ranked hypotheses are repeated target dispatch, bounds/slicing overhead, and missed inlining.
Baseline assembly from the core manifest's size profile shows each packed pixel calling `coordinates`, then `convert_rgb`.
The latter dispatches the runtime target; the packed loop also branches to check pixel/output bounds.
Build command is `cargo rustc --manifest-path crates/ditherette-wasm/Cargo.toml --release --lib -- --emit=asm`.
This uses the core's `opt-level=s`; the measured benchmark manifest uses Cargo's default release optimization.
The inspected assembly therefore does not establish the cause of that benchmark artifact's measured overhead.
The coordinator owns timing; this assignment uses assembly and exactness tests only.

Dispatch once for sRGB and YCbCr, calling the existing table lookup or `srgb8_to_ycbcr` arithmetic.
Use one generic row writer with fixed RGBA/f32x3 array chunks.
The other five spaces continue through `coordinates`, including cylindrical canonicalization.
Converter construction, source helpers, tables, shared color arithmetic, and memory ownership remain unchanged.

## TODOs

- [x] Verify full-image exact bits for all seven spaces, strided input, gray endpoints, hue seams, and output guards.
- [x] Check candidate release assembly, record existing correctness checks, and commit the single candidate.
- [x] Drain owned jobs and remove the local test/build target before handoff.

## Validation

All commands use this worktree's `target/packed-loop` and `CARGO_BUILD_JOBS=2` with default scalar features.

The final candidate passes all 21 tests in this command:

```text
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --release --test prod_packed_color --test prod_quantize --test prod_color --test prod_fields -- --test-threads=2
```

The new packed-boundary test covers all seven spaces at 1x1, 5x7, and 257x3 with packed and padded rows.
It includes all 256 byte grays, colors adjacent to red, varying alpha, and output guards.
Existing tests retain frozen conversion bits, cylindrical canonicalization, legacy color behavior, and field composition.
Source rows already contain whole RGBA pixels; output retains the exact length assertion before chunk traversal.
Validated image dimensions remain nonzero, and source padding stays outside the converted rows.

The final size-profile assembly hoists the sRGB/YCbCr dispatch out of pixel loops.
Those loops contain existing arithmetic and loads/stores without conversion calls or per-pixel bounds branches.
An initial dynamic-chunk form retained iterator calls at `opt-level=s`; fixed array chunks remove those calls.
This is one candidate prepared through assembly inspection, with no benchmark workers or timers.
The coordinator must measure the actual freshly built benchmark artifacts before selecting it.

`rustfmt --check` and `git diff --check` pass. Every owned build and test job has exited.
`cargo clean` removes 1,576 generated files, reporting 692.9 MiB, from this worktree's resolved `target/packed-loop`.
Source and this compact report remain intact; no shared cache is changed.

## Selection

Measured source is `8e09c05d9455af977746060c39d531848eee8b90`.
The fresh benchmark executable is SHA-256 `1390d22c75a477fed99b5ee99daf0e364db7bbf9a8509bb5d2b0f75568564070`.
Against fresh original production `2b9bbd68`, the 58-case selection records 53 pass and five inconclusive cases.
All measured outputs are exact. One predeclared repeat resolves all five inconclusive cases as pass.
The original report remains unchanged. Packed sRGB improves 5.59x and YCbCr improves 6.52x.
All seven forward cases and affected field cases satisfy the agreed selection gates.
CIELAB's repeated forward median increases 8.54%, below the agreed 10% regression limit.
This tradeoff remains visible; this is not a claim that every changed loop becomes faster.

The branch also includes field-only converter reuse from `ad755f85`.
That change improves complete perturb loops 11.91x to 55.52x against original production in the same trial.
It does not include the separately held Yliluoma target-reuse commit.
The final 101-case frozen-spec comparison uses this selected artifact for both roles and preserves every slower-production case.
See [the scalar measurements](83-scalar-measurements.md) for full tables, scopes, samples, and inherited resize differences.
The trusted frozen-reference guard passes, including isolated native and Wasm compilation.
Selection does not clear browser, visual-drift, packaging, publishing, or rollout holds.
