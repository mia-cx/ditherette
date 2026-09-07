# S23 exact mip reuse candidate

The public prepared baseline is `0be73eb64d9baced6a32ff19204dd50cddd7aba3`.
The missing implementation's literal baseline remains `fba85a94afca921a9df15286c26ef4bd22281389`.
Only the coordinator owns this worktree. Other agents own package quantization and benchmark registration elsewhere.

## TODOs

- [x] Reserve and fill one mip chain, retaining the lower level for the upper level's next reduction.
- [x] Verify exact frozen bytes and f32 bits, all reservation failures, and public memory accounting.
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
Wasm compilation and trusted freeze enforcement pass at `07d528a6`.
Frozen spec/image/policy and landed area/bilinear/shared-helper trees match `0be73eb6` byte-for-byte.

## Fixed measurement budget

Five cases run natively and through each of the three browser engines, totaling 80 serial workers.
Two AB/BA pairs use 20 samples, 50 ms warmup, a 250 ms measurement cap, and a 2 ms throughput target.
Cases cover shallow fractional reduction, deep fractional reduction in latency and throughput, integer LOD, and enlargement.
The checked `resize_integration_plan s23` generator defines the exact shapes and opaque fixture.
Both browser roles use the package. The website has no trilinear implementation.
No non-exact timing override is enabled. Use prepared `0be73eb6` plus the same benchmark registration as accepted.
Use candidate `07d528a6` plus registration as candidate. Freeze both artifacts after builds and record their exact revisions.

Benchmark registration uses existing frozen subject `spec:resize:trilinear:mip-area`, not an invented scalar alias.
One native adapter fixture, twelve public protocol fixtures, two budget fixtures, and nine JS protocol fixtures pass.
The full candidate core passes 300 native tests without benchmark features. Both Wasm builds pass.
Eighteen public interface tests and eight private ABI tests pass against the candidate package.
Three-engine installed adapter conformance and fresh artifact preparation remain before measurement.
