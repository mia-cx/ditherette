# S41 packed color loop candidate

Base `ad755f85`, branch `perf/v1-scalar-packed-loop`.
Selection belongs to corrective issue #136 under #83.
This candidate remains unselected until the coordinator records fresh performance evidence.

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
