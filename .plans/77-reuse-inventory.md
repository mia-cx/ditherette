# S35 resize and color row-band reuse

Read S35 and the current execution contract before implementation. Start only after validated S34 is present in ancestry.
This inventory records existing code at the validated S33 base. Recheck the relevant call graph after joining S34.

## Reuse before adding adapters

`prod/tiling` already owns row bands, complete-output partitions, worker capacity, and balanced contiguous assignments.
The frozen `spec/tiling/contract` and `execution` models cover those semantics. Preserve both implementations.
`WorkerBudget` uses half the logical CPUs, clamped to one through eight workers. Pool capacity differs from active workers per job.

Nearest, area, bilinear, and shared convolution already expose absolute-output row-range functions with reusable plans.
They accept the full source and a local full-width output band. Do not crop source coordinates or rebuild their kernels.
Nearest retains its packed word-copy path. Area retains exact integer paths and accepted fractional arithmetic.
Bicubic and Lanczos share convolution; preserve its x-then-y downscale path and identity-axis shortcuts.
Trilinear already has the accepted shared-chain preparation. Do not duplicate mip chains per worker.

## Budgeted integration is the missing part

The current public `PreparedResize` owns fallibly reserved plans and scratch, and accounts for retained capacities.
Its S33 execution reports progress through the existing caller-scratch kernels.
Legacy row-range helpers are not automatically safe public-memory adapters:

- Fractional area allocates a source-width scratch row inside its row helper.
- Bilinear's row helper grows thread-local scratch, which the public instance does not account for.
- Convolution's x-then-y row helper allocates the source-row interval needed for that band's support.

Reuse their arithmetic through caller-owned, preflighted scratch where threading is selected.
Account for every live worker reservation and overlapping support interval before source import and output allocation.
Use disjoint mutable output slices and shared immutable plans/source. Preserve scalar fallback if the policy or budget rejects parallel work.
Progress callbacks remain on the calling thread. They must not execute from Rayon worker closures.
Keep completion and callback-failure cache publication semantics from S33.

## Color distinction

`prod/color.rs` already has a Rayon row-band converter with shared lookup tables and historical thresholds.
That legacy converter writes four floats per pixel. The public package uses packed three-float coordinates with byte alpha.
`prod/color/packed.rs::Converter` reuses landed conversion arithmetic and canonicalizes cylindrical neutral hues.
Reuse that converter for public work. Do not route public results through the legacy four-channel representation.
Current direct quantization converts pixels inline rather than retaining a complete color plane.
Do not introduce a full-image intermediate merely to create a parallel color stage. Coordinate its ownership with S36.

## Evidence

The existing `ditherette-bench` tiling sweep supplies row-band/worker candidates and exact scalar comparison machinery.
Its native general path spawns scoped threads, renders allocated band outputs, then copies them into the final output.
Those timings are not pooled public Wasm timings. Extend existing benchmark subjects for the actual budgeted path.
Use bounded, filter-specific cases and fresh accepted/candidate measurements. Historical M4 color thresholds are evidence to remeasure, not universal constants.
Preserve established bounded reference differences, and require exact threaded-versus-current-scalar outputs before selection.
No scalar kernel rewrite, frozen-spec change, public backend option, or routine PR review belongs to this slice.
