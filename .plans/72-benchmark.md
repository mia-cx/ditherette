# S31 preparation benchmark

## Scope and ownership

Benchmark subjects, protocol, fixtures, and focused tests belong to this worktree.
The coordinator owns joins, role artifact preparation, exclusive measurement, reports, and the final PR.
Production caching belongs to the runtime agent. Source-content hashing belongs to S32.

## Declared matrix

Four workloads each run cold and warm, on native, Chromium, Firefox, and WebKit.
Each of the eight cases uses two alternating accepted/candidate pairs, totaling 128 serial workers.
Each worker collects 20 single-call samples after 50 ms warmup, with a 10-second measurement cap.

| Workload | Source  | Output        | Settings                                          |
| -------- | ------- | ------------- | ------------------------------------------------- |
| Quantize | 32×24   | 32×24 indexed | 256 ordered colors, Lab76                         |
| Resize   | 129×97  | 65×49 RGBA    | Lanczos3, center, scale-aware                     |
| Resize   | 256×192 | 65×49 RGBA    | Trilinear, center                                 |
| Process  | 129×97  | 65×49 indexed | Lanczos2 scale-aware + Floyd-Steinberg, 16 colors |

Cold creates an empty processor outside every call timer and drops it after observing the output.
Warm primes once outside timing with red bytes XOR 255, then restores measured source bytes.
Geometry, palette order, and settings remain equal. The warm instance survives all subsequent calls.
Case identity binds measured bytes. Timed ordinary calls include preparation keys, preparation, input copies, and durable result copies.
No synthetic source hashing is added. Role capabilities explicitly identify uncached versus preparation reuse.
Accepted artifacts retain S30 production; both roles use identical benchmark protocol source.

## Acceptance and budget

Require exact results and no confirmed per-case cold or warm median regression above 10%.
Seek a conclusive warm preparation benefit where setup matters. Filters have no shared latency target.
One accepted/candidate comparison is planned. At most two candidate revisions are permitted.
Repeat only noisy or inconclusive measurements within this budget, after coordinator clearance.

## TODOs

- [x] Add single-call lifecycle hooks with focused validation.
- [x] Add actual Processor subjects, explicit role capability protocol, and browser changed-source priming with focused validation.
- [x] Add the declared matrix generator and hand off clean validated commits.

## Validation

No measurement has run. Local compiler output belongs only to `target/compiler` in this worktree.
Native lifecycle tests cover interactive warmup, discarded calls, sample observation, and failure cleanup.
All four workloads match frozen outputs cold and after changed-source priming.
Browser tests cover changed-source restoration, preflight observation before disposal, and existing timing semantics.
The matrix validates eight cases and rejects throughput or missing cache-state claims.

Final checks passed: 25 focused Rust tests, 30 browser protocol/timing tests, all benchmark examples compile, and `git diff --check`.
The coordinator must join the latest parent corrections before preparing role artifacts.

## Adversarial follow-up

Native verification previously checked only the final result. It now retains the first mismatch from warmup, discarded calls, or samples.
Warm native calls also check measured input against the frozen result after priming, before timing starts.
Browser observations now check source bytes after each single call, catching mutation that a later call could hide.
Focused tests verify failed setup cleanup, transient output evidence, actual cold/warm adapter inputs, and failed browser priming disposal.
Follow-up validation passed 11 Rust tests and 32 JavaScript tests. Earlier protocol checks remain applicable.
All support commits include the actual GPT-6-astra co-author trailer. The branch had no remote head before rewriting.
