# PR #121 bounded ownership follow-up

Parent checkpoint `a3c9629f35280c36e838faa00e9b664b23abcb53`.
The coordinator authorizes two focused review fixes; runtime kernels and frozen files stay unchanged.

## TODOs

- [x] Reproduce retained byte scratch overlap with the existing physical allocation witness; drop old capacity before replacement.
- [x] Independently verify Process parsed diffusion-policy ownership; add exact/one-under coverage and correct accounting.

The agreed seams are native Processor methods, their injected allocation boundary, and the existing GlobalAlloc live-byte witness.
Run each new test red before its fix, then green. No measurements or broad rebuilds are needed.
Only this worktree's new `target/compiler-review` belongs to this follow-up.
Previously deleted compiler targets remain unused. The coordinator owns review replies and downstream S32 joins.

## Byte scratch result

The new native quantize fixture grows the same instance from 10 to 20 pixels at the larger call's exact budget.
The GlobalAlloc witness observes the 80-byte source allocation while the old 40-byte source is still live.
Before the fix, the red assertion observes 2,284 live bytes instead of 2,244.
The default allocator's moving realloc therefore owns 30 bytes above the final 50-byte net growth budget.
Releasing undersized scratch before `allocator.reserve` removes that overlap.
All 13 `prod_processor` tests pass afterward, including the existing diffusion-row physical witness.
Same-capacity warm calls keep their buffers. Growing calls now free then allocate, avoiding the old contents copy.

## Process policy result

`process::run` keeps the original recipe and a separately parsed `DiffusionPolicy` live through execution.
The direct diffusion path counts both; Process omitted the parsed record.
The new test derives its budget from no-dither Process plus three packed rows and `size_of::<DiffusionPolicy>()`.
Before the fix, the one-under call incorrectly returns a complete indexed result.
After the conditional charge, it rejects before reservations/copies, exact succeeds, and no-dither keeps its smaller budget.
All six Process tests pass, including frozen-output equality and the independent separable-converter budget check.

Final focused validation passes 28 tests: Process 6, Processor 13, fields 6, and quantize 3.
Both red and green runs use `cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml`
with `--target-dir target/compiler-review` and the corresponding `--test` selectors.
Kernel arithmetic, preparation identities, metadata, and frozen source remain unchanged.
Process diffusion now requires the additional parsed-record bytes at exact budgets.
Fixed-size benchmark workloads do not exercise retained-buffer growth. Their timing has not been remeasured after these fixes;
the existing S31 report remains bound to `972d4e9a`, not this review-follow-up revision.
