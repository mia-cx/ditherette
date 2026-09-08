# S35-S37 first public row-band matrix

The coordinator approved this fixed 400-worker budget before measurement.
This checkpoint generates plans only. Root owns fresh artifact preparation,
quiet clearance, the shared lease, and serial Chromium/Firefox measurements.

## Commands and controls

Run the example once for each of the nine slice/policy combinations:

```text
row_band_integration_plan s35|s36|s37 two|four|warm NEW_JSON HOST_LOAD_NOTES
```

Each output validates every BrowserCase and the complete experiment. Files use
create-new semantics. The command prints its case and browser-worker counts.
It launches no processing, browser, or timing operation.

Both roles use the actual package, Required threads, and HostWorker execution.
Accepted forces this stage scalar with height 0 and one active worker.
Candidate `two` uses two workers and height 32 for resize/fields, 4 for mixing.
Candidate `four` uses four workers and height 128 for resize/fields, 16 for mixing.
The `warm` plan uses the four-worker policy and one same-call final-output prime
per sample. It measures final-hit overhead, not kernel speedup.

All plans use two alternating pairs, 20 single-call samples, 50 ms warmup, and
a 10-second measurement budget per worker. Each case creates eight workers
across two roles, two pairs, and two engines. The matrix has 8,000 requested
samples. Summed worker measurement budgets are 4,000 seconds; initialization,
oracle verification, and priming add untimed work.

| Slice | Two-policy cases | Four-policy cases | Warm cases | Total cases | Browser workers |
|---|---:|---:|---:|---:|---:|
| S35 resize | 8 | 4 | 4 | 16 | 128 |
| S36 fields | 12 | 4 | 6 | 22 | 176 |
| S37 mixing | 4 | 4 | 4 | 12 | 96 |
| Total | 24 | 12 | 14 | 50 | 400 |

## Workloads

S35 uses nearest 2048×1536→1024×768, fractional area and bilinear
1537×1025→769×513, and scale-aware Lanczos3 2048×1536→512×384.
Each filter has a 129×97→65×49 control under `two`, both candidates on its
large shape, and one large warm final-hit control. Bicubic adds no distinct
scheduling family to this first bounded sweep; native conformance covers it.

S36 reuses all six `row_fields.rs` recipes. Each runs a 33×25 small control and
a larger case under `two`. The sRGB/random recipes use 769×513 large inputs.
The Oklab/blue-adaptive2 recipes stop at 65×49 because existing Firefox evidence
puts that path near 200 ms per call. All six retain one large warm control.
The `four` plan covers all three large sRGB recipes and large Oklab separable.
The latter is deliberately a one-band overhead control at height 128.
Separate Oklab quantize/perturb four-policy timings duplicate components of
that separable call and are omitted from this first sweep.

S37 reuses four predeclared recipes unchanged. Both candidates run their cold
calls. Each recipe has one warm final-hit control. Small heights can use fewer
workers than requested; the declared policy is not a claim of four active bands.

## Correctness and stopping

Every case retains the frozen target oracle and its exact gate. Nearest and all
field/mixing cases require exact frozen output. Landed area/bilinear/Lanczos3
drift uses the existing nonexact diagnostic mode to preserve mismatch evidence
and permit timing. That mode keeps the frozen gate incorrect. It grants no new
approximation approval and changes no tolerance.

Selection requires byte-identical scalar and row outputs from the same artifact.
Keep failures and inconclusive cases. Do not retry to obtain a desired result.
Warm final-hit results can reject added overhead but cannot establish kernel wins.
Further candidate revisions or measurements require coordinator scheduling.

## Validation

- [x] Generator validates every case and experiment through the existing typed protocol.
- [x] Focused example tests check all nine plans, exact worker counts, subject and
  identity reuse, policy roles, warm control uniqueness, required shapes, and
  bounded perceptual dimensions.
- [x] Root generates final plans from the fixed joined revision and runs measurements.

Only this worktree's fresh native `target/compiler` is used for example tests.
No runtime, frozen guard, Wasm artifact, or public backend control changes here.

The completed trials use processing source `5d16c568` and retain all 400 serial workers with 7,215 achieved samples.
All 200 actual production comparisons are exact. `.plans/77-79-public-bench.md` records the report paths and interrupted attempts.
Actual time-capped counts remain valid under the existing minimum-five rule. Inherited bilinear frozen drift and noisy controls stay in S41.
