# Fresh paired performance

`ditherette-bench-pair` is the external coordinator. It prepares immutable copies
of already-built executables, then alternates accepted/candidate order under one
host lease. Each direct `ditherette-bench paired-trial` child exits before the next
starts. There is no baseline-writing or candidate-promotion command.

## Prepare before the quiet phase

Build each requested revision in its own clean worktree with the same toolchain:

```sh
cargo build --manifest-path crates/ditherette-bench/Cargo.toml --locked --release --bins
```

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

The protocol separates single-call latency from calibrated throughput. It also
separates native kernels, complete calls, initialization, and cold/warm application
caches. The current native adapter only supports kernel calls without an application
cache. It rejects other combinations. It excludes fixture decoding, input copies,
hashing, output allocation, and initialization from timing. Future package adapters
must implement complete-call copies/hashing and actual cold/warm cache semantics;
CPU cache scrubbing is not an application-cache reset.

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
