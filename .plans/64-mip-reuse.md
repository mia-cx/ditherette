# S23 exact mip reuse candidate

The public prepared baseline is `0be73eb64d9baced6a32ff19204dd50cddd7aba3`.
The missing implementation's literal baseline remains `fba85a94afca921a9df15286c26ef4bd22281389`.
Only the coordinator owns this worktree. Other agents own package quantization and benchmark registration elsewhere.

## TODOs

- [x] Reserve and fill one mip chain, retaining the lower level for the upper level's next reduction.
- [ ] Verify exact frozen bytes and f32 bits, all reservation failures, and public memory accounting.
- [ ] Benchmark the prepared baseline and candidate under the exclusive lease; retain only measured exact improvements.

The two independent baseline chains calculate the same storage-rounded lower levels from identical source pixels.
Sharing these levels removes duplicate work without changing weights, rounding, LOD, anchors, or intermediate storage types.
Both output buffers remain reserved before source import. All mip buffers remain counted until the prepared record drops.
This candidate changes neither frozen files nor landed resize kernels and helpers.
Measurements wait for every implementation agent, build, and test to exit.
Target a lower fractional-minification cost and lower peak capacity, without slowing integer LOD by more than 10%.
The candidate is unselected until paired evidence establishes exactness and benefit. No release or rollout follows this task.

Six focused native trilinear tests and all eleven public Processor tests pass.
The exact matrix now covers twelve shapes across every anchor, including deeper fractional and integer mip selection.
The memory fixture checks one chain plus two output buffers and verifies actual heap ownership against reported capacity.
Existing failure injection still covers every reservation and releases all partial ownership.
