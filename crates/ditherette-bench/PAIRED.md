# Fresh paired performance

`ditherette-bench-pair` is the external coordinator. It prepares immutable copies
of already-built executables, then alternates accepted/candidate order under one
host lease. Each direct `ditherette-bench paired-trial` child exits before the next
starts. There is no baseline-writing or candidate-promotion command.

## Prepare before the quiet phase

Prepare each requested revision in its own clean worktree with the same toolchain:

```sh
node scripts/prepare-native-benchmark.mjs /absolute/new-native-artifacts /absolute/owned-native-target
```

Preparation cleans only `ditherette-bench`, `ditherette-bench-api`, and `ditherette-wasm`
release outputs in that explicit target. It then builds `--bins --examples --release --locked`.
Dependency caches remain. A shared target can reuse stale local outputs across checkouts;
a clean Git tree alone does not prove that Cargo recompiled the requested source.
Targets must have one owner, and artifact destinations must sit outside their `release` directory.

Preparation copies both binaries into the new artifact directory and makes them read-only.
It invokes each copy's `build-info` command, validates the embedded clean revision and complete
executable SHA-256, and records source inventory and metadata in `build-provenance.json`.
Only a successful handoff contains that provenance file. Failed directories remain diagnostic evidence.
Use the copied worker and coordinator binaries for subsequent preparation and trials.
Examples remain in the build target for untimed experiment generation.

Both binaries support `build-info` under the normal exclusive benchmark execution guard.
It requires no quiet attestation, initializes no registry, and executes no workload.
The JSON contains the existing `BuildIdentity` in `build` and the complete SHA-256 byte array in `executable`.
Preparation still trusts cached external dependencies and the pinned compiler; it is not a hermetic rebuild.

The native executable embeds its full source revision, dirty status, tool version,
and verbose compiler version. A trial rejects a dirty build, a different requested
revision, or a different executable SHA-256. Build artifacts are not inferred from
the working directory during measurement.

Write an `Experiment` JSON using the public typed model. The native control-plan
helper creates two explicitly provisional reference fixtures without running them:

```sh
crates/ditherette-bench/target/release/ditherette-bench-pair \
  control-plan /absolute/new-experiment.json 'Describe unrelated host activity here'
```

The control budget is four AB/BA pairs for nearest single-call and throughput
cases. They use a 512x384 fixture, 256x192 output, 250 ms warmup, 100 samples,
a one-second measurement cap, and a 5 ms throughput calibration target.
These are sequencing controls, not optimization targets or accepted production.

Copy both prepared executables into one fresh directory:

```sh
crates/ditherette-bench/target/release/ditherette-bench-pair prepare \
  /absolute/new-experiment.json \
  /absolute/accepted/ditherette-bench FULL_ACCEPTED_REVISION \
  /absolute/candidate/ditherette-bench FULL_CANDIDATE_REVISION \
  /absolute/new-prepared-directory
```

Preparation reads both executables before writing the manifest. Copies become
read-only. Each trial checks their complete hashes before launch and after exit.
Existing preparation and result directories cannot be overwritten.

## Run after explicit clearance

Preparation reuse cases use `preparation_integration_plan native|public DESTINATION HOST_LOAD_NOTES`.
The helper declares four workloads in both cold and warm states, with two alternating role pairs.
Each worker takes 20 single-call samples, 50 ms warmup, and a 10-second measurement cap.
Running native plus Chromium, Firefox, and WebKit requires 128 serial workers.

Cache metadata declares each role, for example
`{"roles":{"accepted":"uncached","candidate":"preparation"}}`.
Both artifacts must use the same protocol and their declared production implementation.
Cold creates an empty processor before every call timer and disposes it after observing the output.
Warm primes once with every red source byte XOR 255, then restores the measured input.
The retained processor reuses preparation across ordinary calls with unchanged geometry, settings, and palette.
Identity binds measured input bytes. Key preparation and boundary copies remain timed.
These preparation-only artifacts do not hash source content.
Cache cases reject throughput and initialization scopes. Historical `"none"` metadata remains supported.

Image-stage reuse cases use `stage_integration_plan native|public DESTINATION HOST_LOAD_NOTES`.
They use the same worker and sample bounds, with the matrix declared in `.plans/73-benchmark.md`.
Their role metadata declares `"accepted":"preparation"` and `"candidate":"image-stages"`.
Warm cases also declare `sample_prime` inside `roles`, with browser preparation `"primed-sample"`.
Supported primes are `same-call`, `resize`, `perturb`, and `no-dither`.
Each warm call gets a fresh processor and its declared prime outside timing.
This includes warmup, discarded calls, and preflight. Partial-stage cases never accumulate a final-output hit across samples.
Historical `"primed-instance"` retains its once-per-worker, changed-source preparation prime.

Each stage prime's actual output must match its own target-local frozen output before timing starts.
Every measured output is observed before teardown. Failed setup releases its processor and retains the concrete failure.
Native Processor subjects retain their owned results until untimed observation and release prior results outside the next timer.
Their ordinary input copies, production hashes, preparation, and durable output copies remain timed.
Cold cases create empty processors per call and include hashing overhead when the artifact implements it.

Follow [exclusive execution](EXECUTION.md). Drain agents, compilers, builds, and
tests before setting the quiet attestation. Do not invoke Cargo during trials.

```sh
DITHERETTE_BENCH_QUIET=1 \
  crates/ditherette-bench/target/release/ditherette-bench-pair run \
  /absolute/new-prepared-directory/prepared.json /absolute/new-results-directory
```

This coordinator owns its lease directly. Do not wrap it in the single-child lease
helper or launch it from another benchmark. The native worker uses S04's separate
execution lock. Linux `/proc` observations record visible benchmark process counts
before and after measurement. They run outside samples. The execution lock prevents
overlapping ownership throughout measurement, including between observations.

Each launch/reap event records its PID, timestamp, and host load. Raw request,
stdout, and stderr files survive failed spawns, exits, or JSON parsing. Successful
runs also write `report.json`; a missing report means the run did not complete.
Machine identity contains architecture, CPU model, logical CPU count, hostname,
OS, and kernel. This native coordinator currently requires Linux for host evidence.
It does not certify unrelated host quiescence or stop other projects' processes.

## Evidence and decisions

Required cases retain full fixture/settings/artifact digests, dimensions, recipe,
both revisions, warmup settings and observed work, raw per-call samples, and batch
sizes. S05 verifies all three outputs and both executable-local references.
Incorrect results preserve raw output and available PNG review bundles.

The protocol separates single-call latency from calibrated throughput and keeps each
operation's timing scope explicit. Native resize uses caller-owned output storage.
Native complete quantize borrows source bytes and includes preparation, output allocation,
and result destruction. Packed-forward conversion excludes table construction and output
allocation. Its per-iteration output barrier prevents dead-store removal. All native
scopes exclude fixture decoding and verification. Public calls include the actual package
boundary, with no benchmark-only hashing. Historical `"none"` metadata makes no application-cache claim;
explicit role metadata and the lifecycle above describe cache comparisons.

Native quantize and color conformance checks the outputs before and after measurement.
It does not observe every timed output or detect transient A/B/A changes. The typed registry
binds fixed, deterministic in-process callables, with immutable input and no callbacks.
The exact S05 gate covers those checked outputs under that deterministic-kernel assumption.
This mechanism cannot certify arbitrary stateful or nondeterministic subjects. Broader native
observation needs a separately named scope or mechanism; silently retaining quantize results
would remove their destruction from the declared full-call cost.

At least two fresh pairs, in alternating order, are required. The fixed even pair
budget belongs in the experiment before trials. Every required case needs exactly
one accepted and candidate result per pair, compatible tools/settings, at least
five valid samples, one observed benchmark process, and exact S05 conformance.
Missing, duplicate, malformed, or unrelated evidence cannot pass.

A pooled per-case median slowdown above 10% fails only when every alternating
pair independently exceeds 10%. A pass requires every pair below that threshold
and pair-ratio spread within 10%. Mixed thresholds or wider spread are inconclusive.
Retain inconclusive runs and repeat both revisions only within the agreed budget.
Historical timings never replace a fresh accepted role. Remeasurement does not
change either revision or promote candidate code. Pre-freeze conformance remains
provisional even if this performance gate passes.

The coordinator exits zero only for a complete passing comparison. Regression,
inconclusive, incomplete, and incorrect comparisons return exit code 2. Transport
or preparation failures return a nonzero error and retain existing evidence.
