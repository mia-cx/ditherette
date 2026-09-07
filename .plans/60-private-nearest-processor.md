# S19 private nearest processor

The literal-copy baseline is `0ede7f6c6f90d6c5d40b169b1dd835f0ac752902`.
This work adds bounded ownership around its unchanged nearest kernel.
The copied contracts and frozen reference remain unchanged.

## TODOs

- [x] Add the allocation-free failure contract, capacity-accounted processor, and native failure/conformance fixtures.
- [x] Add the module-state Wasm adapter and caught stateless copy helpers; verify actual generated ABI and failure recovery.
- [ ] Validate native/Wasm/JS checks and trusted freeze enforcement; document private ABI and push the integration handoff.

## Boundary decisions

The package agent owns raw request validation and an outer reentry guard.
Rust uses module functions, not generated classes with infallible Rc construction.
The adapter moves processor state out before calling JavaScript, avoiding mutable borrows across callbacks.
All numeric boundary arguments use f64 to prevent generated integer truncation.
The private adapter accepts a borrowed Uint8Array handle, not an owned Rust byte slice.

Account for source/output Vec capacities, owned bookkeeping records, and the pinned externref slab.
Preflight the slab before initialization primes it. A valid tiny limit can fail initialization with memory-limit.
Caught helper failures follow ordinary Rust cleanup. Returned JS storage remains independent of Rust memory.
There is no retained cache or progress callback in this slice. Later publication must follow complete-result construction and callback success.

## Native checkpoint

Five processor fixtures pass, including real first/second allocator failures and allocation-free error reporting.
The three copied contract and three baseline tests also pass without benchmark features.
Command: `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test prod_processor --test prod_contract_baseline --test prod_nearest_baseline`.
All owned files pass scoped rustfmt and `git diff --check`.
That checkpoint pauses for the coordinator's exclusive benchmark window. Implementation resumes only after its owned children exit.

## Private ABI checkpoint

The production processor now mirrors `prod/pipeline/processor.rs`.
The private ABI uses module state and three caught stateless JavaScript helpers.
Six actual release-Wasm fixtures pass with generated web bindings.
Read [the private adapter contract](../crates/ditherette-wasm/src/wasm/processor.md) before integrating its bindings or memory accounting.
