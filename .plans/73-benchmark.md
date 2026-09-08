# S32 image-stage benchmark support

Issue [#73](https://github.com/mia-cx/ditherette/issues/73). Branch `impl/v1-s32-bench`
starts at delivered S31 `a3c9629f35280c36e838faa00e9b664b23abcb53`.
This worktree owns benchmark protocol, development subjects, fixtures, and this plan.
The runtime agent owns production caches. The coordinator owns artifact preparation,
exclusive measurements, reporting, PR integration, and cleanup.

Read the [approved S32 slice](../docs/plans/ditherette-v1/slices.md#s32),
[execution contract](../docs/plans/ditherette-v1/README.md#execution-contract), and
runtime `.plans/73-stages.md` before changing these cases.
Use existing lifecycle hooks, target-local frozen verification, and input identity helpers.
No old compiler cache is assigned. If compilation is needed, use only this worktree's `target/compiler`.

## Declared matrix

The coordinator approved four workloads, each cold and warm, on native, Chromium,
Firefox, and WebKit. Two alternating role pairs yield 128 serial workers.
Every worker takes 20 single-call samples, 50 ms warmup, and a 10-second measurement cap.
Source bytes use the existing S31 deterministic pattern. All resize outputs are 65×49.

| Measured method | Source and settings | Untimed warm prime on the same input |
| --- | --- | --- |
| resize | 129×97, center scale-aware Lanczos3 | Same resize; true final RGBA output hit |
| quantize | 32×24, 256 ordered S31 colors, Lab76, premultiplied alpha | ditherAndQuantize with no dithering; shared final Indexed hit |
| ditherAndQuantize | 65×49, 16 ordered S31 colors, sRGB matching, premultiplied alpha, separable Bayer4 in Oklab, strength 0.7, everywhere | perturb with the same policy; perturbed RGBA hit, cold Indexed stage |
| process | 129×97, center scale-aware Lanczos2 → 65×49, 16 ordered S31 colors, sRGB matching, premultiplied alpha, Floyd–Steinberg byte feedback, strength 0.7, serpentine, everywhere | resize with the same output settings; resized RGBA hit, cold Indexed stage |

The palette generator retains S31's ordered `[n, n*73, n*151]` byte triples.
The source generator retains S31's `[x*17+y*31, x*43+y*7, x*11+y*53, 255]` byte pattern.
Fixture identities name measured bytes and settings, separately from the priming request identity.

## Lifecycle and evidence

Both roles run the identical new protocol. Accepted capability is `preparation`;
candidate capability is `image-stages`. These describe artifact behavior, not a public cache switch.

Cold creates an empty processor outside every timer. Warm creates a fresh processor
and executes its declared prime outside every timer. Each sample therefore starts
with exactly the declared stages, rather than accumulating a final Indexed hit after sample one.
Priming also applies to warmup, discarded calls, and untimed preflight.
Historical S31 `warm`/`primed-instance` records retain changed-source, once-per-worker preparation priming.
The new per-sample lifecycle must have an explicit protocol representation.

Check each actual prime against its own target-local frozen output outside timing.
Observe every measured output before teardown. Retain a concrete mismatching output
or failure diagnostic even if a later call would recover. Failed setup disposes its processor.
Keep the measured source unchanged; validate source bytes after priming and each measured call.

Ordinary input copies, current-source hashing, materialized downstream content hashes,
preparation, and durable result copies stay inside each measured native/public method.
Only fixture construction, instance setup/priming, verification, and teardown stay outside.
Native prior result destruction remains untimed, as in the delivered S31 shared protocol.
No synthetic hash replaces production hashing. Cold hashing/retention overhead is a real measured cost.

## Target and budget

Require exact target-local outputs and report each cold/warm regression gate.
Confirmed per-case median regressions above 10% block the performance gate.
Seek clear warm gains from cached final output and reused materialized intermediate stages;
there is no universal filter latency target. Preserve inconclusive outcomes and cold overhead.
One fresh baseline/candidate comparison is planned. A retry or additional candidate requires
the coordinator's explicit decision within the approved parent budget.
No measurement runs before coordinator quiet clearance and the shared exclusive lease.

## Atomic steps

- [x] Add reusable per-sample priming fixtures and lifecycle tests.
- [x] Extend typed protocol and actual native/public adapters, preserving historical records.
- [x] Add the declared matrix generator and focused frozen exactness checks; hand off for review.

No measurements or artifact preparations have run. Focused Rust checks use only this worktree's `target/compiler`.
No evidence report is copied from S31. Both roles must join the coordinator's corrected S31 parent before artifact preparation.

First checkpoint adds the shared public prime-request mapper and fresh-sample owner.
Three focused Node tests pass. They cover all declared prime mappings, timing boundaries,
fresh ownership for every warmup/sample, and concrete failure propagation with disposal.
The second checkpoint wires those helpers into native/public calls with explicit `primed-sample` metadata.
The isolated oracle derives and checks each prime without changing frozen specification files.
Focused validation passes 40 Node tests, three native observation/cleanup tests, eight frozen-oracle tests, and 12 browser-protocol tests.
Rust tests/examples also pass a locked compilation check. Historical role JSON omits the new optional prime field.

The matrix generator reuses the S31 source/palette helpers and comparable Lab76, Lanczos3, and Process cases.
Five untimed example tests pass, including every native prime/measured output and the isolated oracle's prime derivation.
The protocol guide documents per-sample ownership, timing scope, and historical compatibility.
Artifact preparation, real browser execution, cache-hit evidence, and measurements remain coordinator-owned follow-up work.
