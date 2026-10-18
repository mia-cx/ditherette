# Tiling coverage and tuning

CPU threading and GPU image tiles are independent package controls. Explicit CPU
rows now cover resize, matching, fields, Yliluoma, resolved effects and masks.
Adaptive workers share the full-image selector result. Global recipe statistics
and CPU diffusion remain continuous. GPU diffusion admits the same full error
state and bounds dependency submissions rather than restarting each image band.

The public-worker check compares 381 complete results across scalar whole-image,
required-thread row workers and strict GPU whole/tiled execution. It covers all
resize filters, matching policies, field forms/spaces, diffusion kernels and
feedback/scan combinations, mixtures, effects, masks and composed processing.
The focused regression also exercises cold and retained Adaptive coverage.

## Held benchmark sweep

Mia has not authorized timing execution yet. Remora's GPU residency/work must be
cleared by its owner before a qualified sweep; do not stop it from this task.
No new automatic tile/worker defaults are selected by correctness checks.

After committing the tested source, prepare the matrix without running workloads:

```sh
node scripts/plan-tiling-benchmarks.mjs /private/path/celeste.rgba /private/path/tiling-plan.json
```

The plan binds the full 2600×4168 Celeste RGBA source, Wplace palette, source
revision and all three Wasm hashes. Its catalog is shared with the correctness
worker. Candidate CPU row heights are 8/16/32/64/128/256 with 1/2/4/8 workers;
GPU candidates include whole, auto and those row sizes. Global reductions and
CPU diffusion do not get fictitious independent-worker candidates. GPU diffusion
excludes duplicate sizes above its 64-frontier submission cap.

On Mia's go-ahead, prepare immutable installed-package and harness inventories
using the existing public benchmark preparation, then drain builds/tests/agents.
Run serially under the shared Rust benchmark lease and execution lock. Check
external CPU/browser activity and external GPU processes before every paired
trial and afterward; abort a contaminated pair, retain its evidence, and do not
keep retrying indefinitely. A resident external CUDA owner blocks GPU tuning
even when an instantaneous utilization query reads zero.

Screen by kernel, image/output-size class, matching cost, placement and scan
order before expanding the matrix. Use six fresh alternating pairs for finalists.
Keep decode, initialization, compilation and hashing outside compute timers.
Separate cold computation, source-analysis reuse and final-output cache hits.
Compare every output with the same-engine whole-image control and require actual
GPU/required-thread execution. Choose defaults only from uncontaminated full-call
results; retain whole/auto where evidence is inconclusive. The matrix planner is
preparation, not a measurement executor or a claim of benchmark qualification.
