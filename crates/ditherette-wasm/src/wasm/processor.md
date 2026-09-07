# Private nearest ABI

This is crate-owned package wiring, not an additional public method or recipe.
The package initializes a fresh binding factory and Wasm instance for each processor.
The copied contracts at `0ede7f6c` and frozen `spec/pipeline/processor.rs` supply request, lifecycle, and composition semantics.
`prod/pipeline/processor.rs` adds capacity-accounted ownership around the canonical copied nearest kernel.
`Failure` changes error representation only. Its code/path values allocate no Rust strings.

## Functions

| Private export | Contract |
|---|---|
| `privateInitialize(limit: number): number` | Return zero or a failure status; preflight before priming fixed boundary storage |
| `privateResize(input: Uint8Array, sw: number, sh: number, ow: number, oh: number, anchor: number, sink: object): number` | Borrow both JS handles; write `sink.value` only after complete durable result construction |
| `privateDispose(): number` | Idempotently release processor ownership; reject active-call recursion |
| `privateErrorPath(): number` | Read immediately after a failure status |
| `privateMemoryOverhead(): number` | Private fixture/accounting observation, excluded from the public wrapper |

All incoming numbers use f64 before validation, avoiding generated integer truncation.
Anchors are top-left, top, top-right, left, center, right, bottom-left, bottom, bottom-right, numbered zero through eight.
The wrapper supplies a new private plain `{value: undefined}` sink. It reads the value only after status zero.
Success contains `{width, height, data: Uint8Array}`, with JS-owned data independent of Wasm memory.
Raw request/version/unknown-field validation belongs to the typed package wrapper.
The Rust boundary independently validates dimensions, anchor, intrinsic byte length, and memory limits.
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

## Memory and cleanup

The accounted peak is `privateMemoryOverhead() + input Vec capacity + output Vec capacity`.
Both Vecs use fallible exact reservation before byte copying. Their actual capacities are checked before processing.
The metadata term counts Processor, plan, both Vec headers, module state, error-path storage, and borrowed boundary handles.
Borrowed slice helpers create no extra Rust byte buffer. Returned JS bytes and caller JS storage are excluded under decision37.

The wasm32 release build reports 648 bytes of bookkeeping. A 1x1-to-1x1 call needs exactly 656 bytes.
The 2x1-to-3x2 fixture needs 680 bytes. One byte less fails preflight before any source copy.
Limits from one through 647 are valid option values but fail initialization with memory-limit before priming.
The default remains 1610612736; the maximum remains 2147483648.

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
Caught input/result failures drop both Rust Vecs and restore the ready state.
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

Run after generating web bindings into `dist/private-test`:

```sh
node --test tests/private_processor.mjs
```

Fresh import URLs isolate singleton generated glue only in these low-level Node fixtures.
The shipping package uses its separately tested crate-owned factory, never import-query isolation.
