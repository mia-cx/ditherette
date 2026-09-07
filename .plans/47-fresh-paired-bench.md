# #47 Fresh accepted and candidate performance pairs

Base: `8c05906cb9cfe0a991b351f5260a319b91f76ab4` (S05, PR #96).
Branch: `impl/v1-s06-paired-bench`. PR base: `impl/v1-s05-verification`.

## TODOs

- [x] Define typed paired evidence and per-case regression decisions with deterministic fixtures.
- [x] Prepare immutable artifacts and run sequential alternating children under the shared lease.
- [x] Adapt the native measurement loop, bind builds to revisions, and test transport controls without timings.
- [x] Document preparation, fixed budgets, comparison outcomes, and native measurement limits.
- [x] Prepare distinct native control revisions, obtain explicit quiet clearance, and retain live paired evidence.
- [x] File a non-draft unmerged PR with validation and artifact identities.

## Acceptance and holds

Every required case needs matched settings, exact S05 verification, and fresh raw
samples from both revisions. One-call latency, batched throughput, measurement
scope, and application-cache state stay distinct. Confirmed per-case median
slowdown above 10% fails. Incomplete or noisy evidence cannot pass or promote code.

Prepare both executable copies before measurements. A single external lease
owner runs one direct benchmark child at a time, alternating AB/BA order.
Native controls have no application cache; they cannot claim cold/warm full-call
coverage. Later adapters must implement those explicit protocol modes.

The authorized trial uses two distinct source checkpoints containing the new
transport. Further trials require fresh quiet-phase clearance. Neither checkpoint
becomes accepted production through this task.

Three deterministic comparison fixtures pass. They cover exact threshold,
confirmed slowdown, order-sensitive inconclusive outcomes, missing/duplicate
roles, mismatched identities/settings/toolchains, invalid samples, S05 output
failures, and separate latency/throughput/cache cases. No timings collected.

Coordinator fixture passes with fixed Node responses. Each fake child confirms
the host lease is held and claims an exclusive overlap marker. Events prove
AB/BA starts each follow the previous reap. Nonzero exit, malformed JSON,
existing evidence, writable executables, and changed bytes all fail closed.
This test neither launches the benchmark executable nor collects timings.

Native adapter validation passes without measurement. It rejects unsupported
application-cache/full-call claims and mismatched recipe identities. The build
embeds its source revision, dirty status, and exact compiler version. Runtime
checks bind those values to the complete executable digest before timing.
Process observations bracket measurement; each sample uses the existing loop.
Combined validation passes 4 paired fixtures, 3 binary tests, 11 S05 fixtures,
and S04 Rust/Node ownership fixtures. Criterion compiles and Rust formatting passes.

Accepted control build is `cf132cdd44bb3ba13974544d1c683af92bd2fd22`, clean,
Rust 1.97.0 (2d8144b78), release profile. Executable SHA-256:
`f4c90fb30047ff2267866b30e1f75e7f62f415e9befc6e96852ee4fcce1e0c28`.
Candidate control is `87cff9aaab4a3bb5d033645d21bf1ed615a98455`, also a clean
release build with the same compiler. Executable SHA-256:
`e1671dd7b693e7cb1a30260691ca2dd10613dcd307e42f5be9470703914b937b`.
The candidate includes documentation, checked fixture-length arithmetic, and
reference-drift review bundles. Native reference and measurement code is unchanged.

## Authorized live control trial

Main granted clearance after S15/S16 and the final documentation push exited.
The process check found no compilers, tests, generators, or benchmarks. Unrelated
host activity remained untouched. Main observed lifetime ps CPU of Codex 5.0%,
MainThread 1.1%, Hermes 0.3%, and background services at or below 0.3%.

The prebuilt coordinator ran on 2026-09-07 at 12:43:39 UTC. The final child exited
8.84 seconds after the first launch event. The four-pair budget produced 16
sequential children and 1,600 raw samples. All 16 start events match reap events;
every child reports one visible live benchmark process. All S05 proofs are exact
but pre-freeze. Both OS locks were reacquired after exit, and ps found no remaining
benchmark or coordinator. No browser ran. Main received the lease-release report.

The performance gate returned `regression` and exit code 2:

| Native control case | Accepted median ns | Candidate median ns | Change | Decision |
|---|---:|---:|---:|---|
| One-call nearest | 85,092 | 93,756 | +10.1819% | Inconclusive |
| Throughput nearest | 86,792.4741 | 95,887.9057 | +10.4795% | Regression |

Throughput pair ratios are 1.1020005823, 1.1025781461, 1.1141181454, and
1.1019189446. Every alternating pair exceeds 10%. Latency ratios cross the
threshold, so latency stays inconclusive. The binary performance difference is
not diagnosed here. The coordinator's rejection is retained, not converted into
a passing control or an optimization claim. No retry or promotion occurred.

Machine: athena-hephaestus, Linux 6.12.95+deb13-amd64, x86_64,
AMD Ryzen 9 7950X, 24 available logical CPUs. Per-event load averages remain in
the journal. Both cases retain the same complete source and settings identities:

- Input SHA-256: `cea06d0b2dd9e420f4bf05661f21b0c3715aa635411724ea608796bd84da85df`.
- Settings SHA-256: `a6273fdc2c9ff8522fe9883526d2f4154be7defa3c45cb7a87985d6bbe8be718`.

All local raw evidence is retained beneath this worktree's
`crates/ditherette-bench/target/s06-controls/trial-01/`. It contains the complete
prepared manifest, journal, report, and 16 request/result/stderr sets. Immutable
executables remain in sibling `prepared-final/{accepted,candidate}/ditherette-bench`.
These generated files stay ignored; this record binds their retained contents:

- `report.json`: `44f9464134438271fc0875d1589b70822c3cfa0814fd3ea8daec36397797ee76`.
- `events.jsonl`: `fcd7897e12708cf646942649e7941a63b02ca68d18187b80d0113e7b30e78ef4`.
- `prepared.json`: `e1f66044133b9a0632260d57827cfaafcd3a68e878117ee383ebdbe3b7c2e5c8`.

Delivered in [PR #102](https://github.com/mia-cx/ditherette/pull/102), non-draft and
unmerged against `impl/v1-s05-verification`. Final evidence commits change no
measured implementation. The control candidate stays rejected for performance;
this delivery makes the paired measurement tooling available, not that candidate
accepted production.
