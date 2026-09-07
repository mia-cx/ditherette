# S20 first browser trial, incomplete

## Preparation

Clean source revision `b11a14a6b4fca50366a83dca81bfed5aff812db3` builds the release worker and both package roles.
Both independently packed tarballs have SHA-256 `bed93cd2085df64a2ca8ba578fd6d72babccc539042e83847e66a691bde59c1d`.
Build/source manifests remain under `/tmp/ditherette-s20-matched.wwwToe/{accepted,candidate}`.
All three engine snapshots and the fixed experiment remain under `crates/ditherette-bench/target/s20-public-trial-01`.
Preparation uses the complete runtime, alias-preserving file groups, actual TypeScript closure, and frozen output preflight.

Every implementation agent, compiler, build, test, and preparation process exits before measurement starts.
The sole coordinator runs Chromium with the predeclared four-pair, nine-case budget.
Firefox and WebKit measurements have not started.

## Retained outcome

The run stops at `pair-003-case-003-accepted`, the actual TypeScript reduction-throughput role.
Its request contains 14,237,948 bytes of JSON, including source and frozen-output arrays.
Node PID 683687 reaches about 1.5 GiB RSS. Chromium's renderer disappears while its browser and transport remain alive.
The session cgroup reports `oom_kill 1`; kernel logs are unavailable to this user.
This supports investigating memory pressure but does not independently identify which process the kernel killed.

After inspecting live process state, the coordinator sends SIGTERM only to the owned Node transport.
Its cleanup closes Chromium; the worker and coordinator exit unsuccessfully. No benchmark or browser children remain.
The coordinator exits 1 and preserves every request, output, stderr file, and lifecycle event.

The retained lifecycle contains 62 starts and 62 reaps, with maximum concurrent workers 1.
There are 61 complete result records and one empty failed result file.
All 61 outputs match their frozen reference bytes and metadata, covering 6,100 samples with no zero samples.
No complete report exists. Partial correctness does not establish a performance gate, accepted optimization, or slice readiness.

## Next action

Use an untimed transport fixture to test the memory cost of passing large numeric arrays through Playwright RPC.
If confirmed, move bulk request/result data to bounded loopback HTTP transfer, keeping only compact control messages in Playwright.
Retain manifest routing restrictions, exact input/output checks, runtime identity, and owned-child cleanup.
Propagate renderer failure without leaving the transport silently waiting.
Rebuild both roles and prepare new snapshots after any transport correction. Never combine this run with fresh samples.
The fixed case/pair budget remains unchanged; this failed transport run is incomplete evidence, not an optimization attempt.

The obsolete, separate-file WebKit diagnostic copy now lives at `crates/ditherette-bench/target/s20-retained-runtime-dxp7eD`.
Moving its 550 MiB off RAM-backed `/tmp` preserves every file. The measured alias-preserving snapshots remain unchanged.
