# Same-kernel diagnostic budget

This experiment investigates the S20 Firefox regression. It does not select an optimization or replace release gates.
Read [the public-call report](61-public-measurement.md) for the observed regression and retained evidence.

## Fixed experiment

Use the same varied RGBA fixture and center-nearest dimensions as S20 reduction, 1024×768 to 512×384.
Run Chromium and Firefox sequentially. Each engine receives these six cases.

| Compared roles | Latency scope | Throughput scope |
| --- | --- | --- |
| Actual TypeScript / unchanged public package | Complete call | Complete call |
| Canonical kernel / canonical kernel | Preallocated kernel call | Preallocated kernel call |
| Legacy kernel / legacy kernel | Preallocated kernel call | Preallocated kernel call |

Both package and diagnostic calls use the same developer-only Wasm build.
Normal package builds exclude diagnostic exports. The public API and production dispatch remain unchanged.
The diagnostic export still incurs its small JS/Wasm call and Rust dispatch costs.
Its scope excludes source/output allocation and durable result transfer, but does not claim an entirely internal Wasm timer.
The legacy convenience kernel retains its own per-call plan construction.

Use two alternating pairs, 50 samples per worker, 250 ms warmup, and a 500 ms measurement cap.
Throughput calibration targets 5 ms per sample. Latency executes one call per sample.
The maximum is 48 sequential workers and 2,400 retained samples.
Keep zero/coarse-clock samples and incomplete gates. Do not repeat an unchanged run to seek a preferred verdict.

## Interpretation and execution

The unchanged public path must reproduce the slowdown in the diagnostic binary before attributing a difference to removed boundary work.
Feature-gated exports can change compiled layout, so a missing reproduction is a finding, not proof that the original result was wrong.
Canonical and legacy controls have the same declared scope; their absolute times can guide the next hypothesis.
Never pass a mixed kernel/public pair through a single performance gate or call its ratio an accepted optimization.
Verify full output bytes before timing and after the run through the existing frozen-reference checks.

Prepare clean role artifacts, runtime snapshots, and source identities before entering the quiet phase.
All diagnostic and resize-baseline agents, compilers, builds, and tests must stop before measurement.
Use the existing exclusive paired coordinator and retain every worker's request, result, stderr, and launch/reap evidence.
Resume implementation only after all owned descendants exit.
