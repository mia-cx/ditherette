# S19 prerequisite preparation

Issue [#60](https://github.com/mia-cx/ditherette/issues/60) requires S02, S06, and S18.
This file records the validated integration base. Production begins in a separate child worktree from this checkpoint.

## Join

The pre-existing `impl/v1-s19-base` branch pointed to S02 when its worktree was materialized.
The coordinator merged S17 and S06 without conflicts, producing `ad2410b481d710fb2d739695ec1637d6d08eab79`.

| Prerequisite | Included delivered commit |
| --- | --- |
| S02 package/build ownership | `bc110d91d441ec3069d128e3cc7d39b3b439a1f4` |
| S06 paired benchmark tooling | `a65f53e24b880932021b39e601b5682b1111bc54` |
| S17 complete reference, pending S18 freeze | `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` |

Each commit passes `git merge-base --is-ancestor` against the join.
`spec/`, `image/`, the Wasm Cargo lockfile, and the benchmark Cargo lockfile are unchanged from S17.
Resolved serde_json features remain alloc/default/std for the core test graph and default/std for the benchmark graph.
Neither graph enables preserve_order.

## Validation

All commands ran in this isolated worktree at the joined code checkpoint.
No actual benchmark measurement ran.

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects` passed.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects` passed.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` passed.
- `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test reference_subjects --test verification --test verification_adapters --test lease --test paired` passed.
- The combined benchmark run includes 4 binary, 4 paired, 5 reference-subject, 8 verification, and 3 adapter tests. The lease lifecycle fixture passes, including its three owned Node transport checks.
- `cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --benches` passed.
- `cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml --check` passed.

## Completed freeze prerequisite

- [x] Merge the delivered S18 head into this join.
- [x] Run its trusted freeze guard against this resolved dependency/build graph.
- [ ] Record all prerequisite SHAs, remove only satisfied native blocking edges, and start the S19 implementation worktree.

All PRs stay unmerged. No production copy or optimization has started.

The final delivered S18 head is `eee0b5ddfb600b9ba6517c3dcb755e3566fc7813` in PR #104.
Its reviewed guard passes against the S19 join at `363324c43556f08ef4e8677d226f5659977bcc66`.
That run uses the separate S18 trusted checkout, not a guard selected by candidate files.
It validates frozen content, the corrected symbol/macro/raw-identifier checks, four dependency contexts, and five isolation compilations.
The final documentation-only merge is `1f7e7a68803f5af6953ec710c536cbb66a32c1f8`.
Its crates, guard, and workflow trees are identical to the validated join.
All three required delivered heads are verified ancestors. S18 is no longer a missing code prerequisite.

## Read-only allocation preflight

The nearest literal-copy closure is `spec/resize/scalar/nearest.rs`, `spec/resize/common/alignment.rs`, and `spec/contract/{request,error,lifecycle}.rs`.
Copy these into mirrored `prod/` paths with mechanical wiring changes only, then validate the baseline in its own commit.
The inherited optimized `prod/resize/scalar/nearest/` directory must move behind an explicit candidate module path.
It cannot remain the accepted implementation merely because it already exists.

The nearest kernel allocates nothing. Its future production adapter can reserve fallibly and use frozen `ImageBuf::from_vec_packed`.
Record actual capacity separately because `ImageBuf` exposes length, not capacity.
Count input/output Wasm capacities and bookkeeping/boundary capacities. Plans and retained entries add their own capacity.
Keep allocation-failure reporting free of fresh Rust string allocations.

The S02 generated legacy glue copies incoming slices through `__wbindgen_malloc` before Rust preflight.
Its returned `Vec` passes through `into_boxed_slice`, then JavaScript copies with `.slice()` before freeing.
That generated sequence has no `finally` for a thrown JS copy. It is unsuitable for the new bounded-memory call.

The audited alternative uses an externref `Uint8Array` input and two private `#[wasm_bindgen(catch)]` imports.
One helper copies into an already reserved borrowed Rust slice. The other constructs the complete durable JS result.
Outbound borrowed slices use pointer/length ABI in pinned wasm-bindgen 0.2.121 without another owned allocation.
Raw js-sys `copy_to` and `copy_from` lack catch declarations, so the new boundary must not use them unguarded.
Keep Rust buffers owned until each helper returns. Caught failures restore lifecycle state through ordinary Rust cleanup.
Create output bytes and metadata before the completion callback. Publish cached state only after that callback succeeds.

Validation must cover exact budget boundaries, rejected preflight before any source copy, each allocation/copy failure, recovery, reentry, disposal, and durable isolated results.
The smallest initial candidate is an identity-copy fast path. Inherited planned candidates need fallible metadata allocation first.

## Read-only instance-loading preflight

S02's generated `crates/ditherette-wasm/dist/scalar/ditherette_wasm.js` declares shared `wasmModule`, `wasmInstance`, `wasm`, and cached views.
Its initializer returns existing `wasm` on later calls, even when callers supply another custom module.
Concurrent initialization can race before that shared value exists. Separate Rust processor objects alone do not isolate Wasm memory.

Keep normal generated web glue intact for inherited paths. The crate-owned build should emit a private binding factory beside it.
Use an AST-based export rewrite with a generation invariant/test. Preserve static stateless copy-helper imports at module scope.
Move mutable glue state, cached views, helpers, exported functions, and classes into each factory invocation.
Each `createDitherette` call gets fresh binding closures and a fresh Wasm instance/memory.
Compiled modules may be shared. Initialized glue, import closures, and memories may not.
Avoid eval, Blob imports, and URL-query cache busting.

The root package import stays inert. Initialization lazily imports only the selected scalar factory and defaults to package-relative Wasm assets.
Forward custom input through the actual generated `{ module_or_path: input }` initializer shape.
Its pinned declarations accept RequestInfo/string, URL, Response, BufferSource, and WebAssembly.Module.
Preserve typed-view offsets. Handle Response consumption deliberately; an earlier create must not poison a later instance.
Map initialization failures to structured errors and discard that failed factory without changing other instances.
Disposal releases Rust ownership once and drops wrapper references. It does not claim Wasm pages shrink.

Package staging currently follows typechecking. Stage generated factory declarations before wrapper compilation instead.
An initial layout candidate uses source/dist `rootDirs`, retaining emitted package-relative `dist/wasm` imports.
Verify actual packed-package resolution in package-owned browser fixtures; this layout is not yet tested.

The loading owner will cover crate build/factory generation, package index/scalar/types/errors/validation, staging, and initialization fixtures.
The allocation owner covers the separate Rust processor and caught-copy helpers. The baseline owner covers the five literal copies.
Only the coordinator reconciles their shared build/export registration changes.
