# S22 landed convolution integration

Start from restoration `467542f49ce3f600e5b03aeef574a97be554ae15`.
Mia's #108 direction preserves existing production kernels and shared code. Historical copied-baseline records are superseded evidence.

## TODOs

- [x] Add fallible capacity-accounted plans and caller-owned scratch while preserving landed arithmetic and dispatch; prove original-output and failure behavior.
- [~] Validate native/Wasm compilation and frozen independence, commit/push the native support, then await the S21 public integration checkpoint.

## Ownership and seam

This phase owns only convolution/bicubic/Lanczos modules, focused tests, and this record.
S21 owns shared allocation helpers and the initial public processor/package seam. The coordinator owns benchmark code, joins, and PR filing.
Use S21's `CapacityBudget` with compact `Failure`, counting actual vector capacities and rejecting reservation failures.
Mode plans expose conservative metadata/scratch requirements, fallible construction, actual metadata capacity, and scratch length.
Execution accepts caller-owned scratch and uses the landed pixel writers and branch selection.

Fixed Lanczos2/3 keep their const-radius filter dispatch. Scale-aware minification keeps full-image x-then-y scratch above its landed threshold.
Anisotropic and identity branches remain unchanged. No no-scratch fallback, new arithmetic, benchmark, or optimization is authorized.
Public wiring follows only after a validated S21 checkpoint and coordinator-owned join.

## Native support

`BicubicResizePlan` adds `required_bytes`, `try_new`, `capacity_bytes`, and `scratch_elements`.
`LanczosResizePlan` adds the same accounting methods and `try_new2`/`try_new3` constructors.
The Lanczos requirement method takes the radius; its constructors preserve const-radius fixed-policy filters.
Both modules export `resize_*_rgba8_with_plan_and_scratch_into`, returning compact `Failure`.

Requirements include nested vector headers, conservative tap capacity, and the existing full-call f64 scratch.
The caller separately accounts for inline plan storage and input/output allocations.
Construct with one `CapacityBudget`, then reserve and initialize scratch through that same ledger.
Discard the preparation and its ledger together on failure. `capacity_bytes` excludes caller-owned scratch.
Identity plans have no heap storage; existing cached row-band APIs also accept these plans.
The checked full-call API rejects mismatched dimensions, stride, backing length, or short scratch before output writes.

The original one-shot APIs retain their owned scratch path. Pixel writers and reconstruction filters are unchanged.
The full-call kernel only receives optional scratch; its branch order, threshold, and accumulation loops stay unchanged.
Fallible tap construction keeps the original coordinate, weight, clamping, and contribution order.
Legacy row-band execution retains its allocations; it is outside this scalar public-call integration.

Five new tests pass. The matrix covers 378 exact landed-output comparisons across all anchors, both support policies,
identity, odd shapes, single-axis resizes, singleton input, and the large-image separable branch.
Each matrix execution records zero allocations. Fixtures also cover every plan/scratch reservation failure,
successful retry, insufficient budgets, overflow, short scratch, malformed views, and identity row bands.
The four existing independent frozen-oracle and row-range tests also pass.

The landed separable path already documents bounded frozen-oracle behavior for large downscales.
Exact agreement with landed output does not approve a new approximation or establish universal frozen equality.
No benchmark, public package, or browser runtime result is claimed in this native checkpoint.
