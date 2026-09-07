# S22 landed convolution integration

Start from restoration `467542f49ce3f600e5b03aeef574a97be554ae15`.
Mia's #108 direction preserves existing production kernels and shared code. Historical copied-baseline records are superseded evidence.

## TODOs

- [~] Add fallible capacity-accounted plans and caller-owned scratch while preserving landed arithmetic and dispatch; prove original-output and failure behavior.
- [ ] Validate native/Wasm compilation and frozen independence, commit/push the native support, then await the S21 public integration checkpoint.

## Ownership and seam

This phase owns only convolution/bicubic/Lanczos modules, focused tests, and this record.
S21 owns shared allocation helpers and the initial public processor/package seam. The coordinator owns benchmark code, joins, and PR filing.
Use S21's `CapacityBudget` with compact `Failure`, counting actual vector capacities and rejecting reservation failures.
Mode plans expose conservative metadata/scratch requirements, fallible construction, actual metadata capacity, and scratch length.
Execution accepts caller-owned scratch and uses the landed pixel writers and branch selection.

Fixed Lanczos2/3 keep their const-radius filter dispatch. Scale-aware minification keeps full-image x-then-y scratch above its landed threshold.
Anisotropic and identity branches remain unchanged. No no-scratch fallback, new arithmetic, benchmark, or optimization is authorized.
Public wiring follows only after a validated S21 checkpoint and coordinator-owned join.
