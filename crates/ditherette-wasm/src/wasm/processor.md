# Private scalar processor ABI

This is crate-owned package wiring, not an additional public method or recipe.
The package initializes a fresh binding factory and Wasm instance for each processor.
The copied contracts at `0ede7f6c` and frozen `spec/pipeline/processor.rs` supply request, lifecycle, and composition semantics.
`prod/pipeline/processor.rs` adds capacity-accounted ownership around the landed nearest, area, bilinear, bicubic, and Lanczos kernels.
`Failure` changes error representation only. Its code/path values allocate no Rust strings.

## Functions

| Private export | Contract |
|---|---|
| `privateInitialize(limit: number): number` | Return zero or a failure status; preflight before priming fixed boundary storage |
| `privateResize(input: Uint8Array, sw: number, sh: number, ow: number, oh: number, algorithm: number, anchor: number, support: number, sink: object): number` | Borrow both JS handles; write `sink.value` only after complete durable result construction |
| `privateQuantize(input: Uint8Array, width: number, height: number, palette: number[], matching: number, alphaMode: number, threshold: number, matte: number, sink: object): number` | Borrow input, compact palette, and sink; publish complete JS-owned indexed output after every copy succeeds |
| `privateDispose(): number` | Idempotently release processor ownership; reject active-call recursion |
| `privateErrorPath(): number` | Read immediately after a failure status |
| `privateMemoryOverhead(): number` | Private fixture/accounting observation, excluded from the public wrapper |

All incoming numbers use f64 before validation, avoiding generated integer truncation.
Algorithm tags are `0` nearest, `1` area, `2` bilinear, `3` bicubic, `4` Lanczos2, and `5` Lanczos3.
Other values return invalid-settings at `output.resize`.
Anchors are top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right, numbered zero through eight.
Every mode except area validates the anchor. Area requires raw anchor zero; the wrapper rejects public anchor fields.
Support follows anchor in the ABI: `0` fixed and `1` scale-aware for bicubic/Lanczos.
Nearest, area, and bilinear require raw support zero and reject public support fields.
Bicubic/Lanczos public requests require explicit support. Invalid raw support returns invalid-settings at `output.resize`.
The wrapper reports invalid public support at `output.resize.support` before entering Wasm.
Area retains fractional-overlap integration and its landed fast paths. Bilinear widens triangle support during minification.
Convolution keeps fixed radii or widens support during minification according to the selected policy.
The landed accumulation and clipping/rounding paths remain unchanged, including their documented bounded reference differences.
The wrapper supplies a new private plain `{value: undefined}` sink. It reads the value only after status zero.
Success contains `{width, height, data: Uint8Array}`, with JS-owned data independent of Wasm memory.
Raw request/version/unknown-field validation belongs to the typed package wrapper.
The Rust boundary independently validates dimensions, algorithm, anchor, support, intrinsic byte length, and memory limits.
Supplied progress remains explicitly unsupported until S33.

Status zero means success. Statuses one through thirteen follow the copied error categories:
invalid-request, invalid-image, invalid-palette, invalid-settings, unsupported-operation, capability, initialization,
memory-limit, wasm-memory-unavailable, disposed, reentrant-call, callback, runtime.

| Path ID | Public path |
|---|---|
| 0 | instance |
| 1 | memoryLimitBytes |
| 2, 3, 4, 5 | source.width, source.height, source.data, source |
| 6, 7, 8 | output.width, output.height, output |
| 9, 10, 11, 12 | output.resize.anchor, wasm, control, output.resize |
| 13, 14, 15, 16 | palette, alpha, alpha.threshold, matching |

## Direct quantization

Matching tags `0` through `4` select Euclidean sRGB, linear RGB, Oklab, CIELAB, and YCbCr respectively.
Other raw matching values return unsupported-operation at `matching`.
Alpha mode `0` preserves using the f64 threshold, `1` uses premultiplied RGB, and `2` uses a matte.
Preserve requires matte zero. Premultiplied requires threshold and matte zero. Matte requires threshold zero.
RGB integers encode `0xRRGGBB`; `16777216` identifies Transparent in the compact palette only.
The wrapper validates every caller palette entry, then passes at most 257 codes.
Rust retains a fixed 257-entry record, preserving the distinction between a full palette and a truncated one.
The prepared palette retains at most 256 entries, stable indices, and frozen warning text.

Success contains `{width, height, indices: Uint8Array, palette: {rgba: Uint8Array, transparentIndex: number | null}, warnings}`.
The caught void completion import constructs both durable byte arrays and all warning objects before assigning the sink.
An exception leaves the sink unpublished, drops call-owned Rust allocations, and restores the shared ready state.
Dimensions, palette codes, alpha settings, matching tags, and intrinsic input length are independently validated in Rust.

## Memory and cleanup

The resize peak is `privateMemoryOverhead() + prepared heap capacity + input Vec capacity + output Vec capacity`.
Prepared heap capacity includes the selected plan's allocations and any f32 area/bilinear scratch.
Convolution also counts every nested tap-vector header, tap capacity, and selected f64 full-call scratch.
The complete planned capacity preflights before allocation. Plan, scratch, and both image Vecs reserve fallibly before source copying.
Actual vector capacities are checked against the same limit. Execution uses those owned buffers without further allocation.
Bookkeeping counts Processor, request and prepared-plan records, both image Vec headers, module state, error-path storage, and borrowed boundary handles.
Inline records count once; nested heap headers and data belong to prepared capacity. Identity calls need no prepared heap storage.
Borrowed slice helpers create no extra Rust byte buffer. Returned JS bytes and caller JS storage are excluded under decision37.

Bookkeeping depends on the compiled record layouts. Fixtures read `privateMemoryOverhead()` from their actual Wasm artifact.
They add the selected plan/scratch and image capacities to derive exact/one-under budgets; do not reuse historical byte totals.
The 1x1 identity fixture needs that observed overhead plus eight image bytes, with no plan heap.
See `tests/private_processor.mjs` and `packages/ditherette/tests/public.test.mjs` for mode-specific capacity fixtures.
Positive limits below the observed overhead are valid option values but fail initialization with memory-limit before priming.
The default remains 1610612736; the maximum remains 2147483648.

Quantize adds its request record, fixed palette-entry array, and borrowed boundary record to shared bookkeeping.
Its peak also counts the full prepared quantizer, source Vec capacity, and index Vec capacity.
Prepared capacity includes inline conversion tables, visible matching coordinates, normalized palette bytes, and warning-string capacities.
The complete requirement preflights before preparation or source import. Every heap reservation is fallible and actual capacity is checked.
Quantization allocates nothing after importing source bytes. No float alpha plane or intermediate RGB image is retained.
`tests/private_quantize.mjs` and `tests/prod_processor_quantize.rs` cover budget failures, caught copies, and recovery.

Bookkeeping conservatively includes wasm-bindgen 0.2.121's fixed first 128 usize externref slots, totaling 512 bytes.
Initialization preflights before creating and dropping one numeric JsValue to prime this storage.
A trap specifically during privateInitialize maps to wasm-memory-unavailable and discards the failed factory.
The generated Wasm initializer remains a separate package loading phase.

Dispose drops all processor-owned images/cache/scratch. This slice retains none between calls.
The fixed wasm-bindgen slab is module runtime bookkeeping and has no shrink API.
The wrapper drops factory references on disposal; neither Rust nor the wrapper claims Wasm pages shrink immediately.

Module state moves the Processor into the active Rust call before invoking JavaScript.
No RefCell or generated class borrow spans an imported helper.
Reentrant processing, initialization, and disposal return structured failures without changing the active call.
Caught input/result failures drop the prepared plan, scratch, and both image Vecs, then restore the ready state.
Future cache publication must follow complete result construction and any successful completion callback.
This slice publishes no cache entries.

## Verified binding behavior

Generated privateResize directly calls Wasm with borrowed externrefs. It contains no malloc, byte slice, or owned-handle insertion.
The generated input/output imports call getArrayU8FromWasm0 on reserved borrowed slices inside handleError.
Input length uses captured TypedArray intrinsics, ignoring caller-defined length properties and preserving view offsets.
Copy helpers deliberately remain stateless; normal prototype methods permit deterministic host-failure fixtures.

The first attempted completion import returned Result<JsValue, JsValue>.
With pinned bindings, a thrown helper leaked its undefined owned return slot even though Rust consumed the exception.
Each result-copy failure retained one slot; 512 failures grew table capacity from 1156 to 2052 entries.
The corrected completion helper returns void and assigns a borrowed private sink only after constructing the whole result.
No processing import returns an owned JsValue.

The actual release-Wasm fixture runs 512 cycles of success, caught input-copy failure, and caught result-copy failure.
Table capacity remains 1156 after initialization's 1028-to-1156 prime.
Live slots and memory page count remain unchanged. Every failed call leaves its result sink empty and permits recovery.
Additional fixtures cover exact/one-under budgets, invalid/tiny limits, detached and offset inputs, hidden RGB,
durable output after memory growth/disposal, failed sink assignment, independent instances, and recursive calls.
The indexed boundary also runs 512 success/failure cycles without growing externref capacity, live slots, or memory pages.
Its borrowed index and palette copy lengths use captured typed-array intrinsics.

Run from the crate directory; the script builds the scalar bindings in `dist/scalar` before checking them:

```sh
pnpm test:private
```

Fresh import URLs isolate singleton generated glue only in these low-level Node fixtures.
The shipping package uses its separately tested crate-owned factory, never import-query isolation.
