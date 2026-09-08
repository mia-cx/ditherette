# PR #121 bounded ownership follow-up

Parent checkpoint `a3c9629f35280c36e838faa00e9b664b23abcb53`.
The coordinator authorizes two focused review fixes; runtime kernels and frozen files stay unchanged.

## TODOs

- [x] Reproduce retained byte scratch overlap with the existing physical allocation witness; drop old capacity before replacement.
- [ ] Independently verify Process parsed diffusion-policy ownership; add exact/one-under coverage and correct accounting.

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
