# #47 Fresh accepted and candidate performance pairs

Base: `8c05906cb9cfe0a991b351f5260a319b91f76ab4` (S05, PR #96).
Branch: `impl/v1-s06-paired-bench`. PR base: `impl/v1-s05-verification`.

## TODOs

- [x] Define typed paired evidence and per-case regression decisions with deterministic fixtures.
- [x] Prepare immutable artifacts and run sequential alternating children under the shared lease.
- [x] Adapt the native measurement loop, bind builds to revisions, and test transport controls without timings.
- [ ] Prepare distinct native control revisions, obtain explicit quiet clearance, and retain live paired evidence.
- [ ] File a non-draft unmerged PR with validation and artifact identities.

## Acceptance and holds

Every required case needs matched settings, exact S05 verification, and fresh raw
samples from both revisions. One-call latency, batched throughput, measurement
scope, and application-cache state stay distinct. Confirmed per-case median
slowdown above 10% fails. Incomplete or noisy evidence cannot pass or promote code.

Prepare both executable copies before measurements. A single external lease
owner runs one direct benchmark child at a time, alternating AB/BA order.
Native controls have no application cache; they cannot claim cold/warm full-call
coverage. Later adapters must implement those explicit protocol modes.

Actual trials remain pending the main coordinator's quiet-phase clearance.
Use two distinct source checkpoints containing the new transport for the live
control proof. Neither checkpoint becomes accepted production through this task.

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
