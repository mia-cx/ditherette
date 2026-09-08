# S32 shared image-stage cache

Issue [#73](https://github.com/mia-cx/ditherette/issues/73). Preparation branch
`impl/v1-s32-stages` starts at S31 checkpoint
`4aad1dbe5f3eea46d7e5d04572ac5dffae23fba9` in `.worktrees/v1-s32-stages`.
The coordinator authorized implementation after S31 runtime
`972d4e9a5882b25bca3de5f0786ad1525b5e6329` passed its exclusive trial.
The branch joins corrected S31 PR #121 at
`59b1fe3acdbeae27bbb8ab780b46d2b6b9d67a76`. Its reviewed follow-up releases
undersized byte scratch before replacement and counts Process's parsed diffusion policy.
S32 now owns its assigned runtime files and tests.

The validated S31 handoff advances the dependency gate. The inventory below still
matches its actual preparation owners; final provenance is joined.
The eventual PR base is the delivered S31 branch, currently `impl/v1-s31-preparation`.
Native compiler output belongs to `target/compiler` inside this S32 worktree.
Fresh Wasm compiler outputs belong to this worktree's
`crates/ditherette-wasm/target/scalar` and `target/threads`; both were absent before
the coordinator approved their use. Generated crate/package `dist` directories
are local staging output. Existing shared compiler paths are not used.
Build/test jobs may run here; measurements require coordinator clearance.

## Authority and reuse

Read the [execution contract](../docs/plans/ditherette-v1/README.md#execution-contract),
[S32 slice](../docs/plans/ditherette-v1/slices.md#s32), and
[decision #37](https://github.com/mia-cx/ditherette/issues/37#issuecomment-5562808049)
before implementation. For identity, ownership, pressure, and publication, read
`crates/ditherette-wasm/src/spec/contract/cache.md` and `cache.rs` completely.
Use `tests/spec_contract_cache.rs` for independent identity and ownership fixtures.
Read frozen `spec/contract/lifecycle.rs` when connecting completion and failure.

The existing `prod/contract/cache.rs` is already byte-identical to the frozen
reference at this checkpoint. It includes source/stage identities, all five
request compositions, capacity accounting, LRU, and pending publication. Reuse
this literal baseline and its fixtures; another copied cache model is unnecessary.
The production runtime still has no image-stage owners or image-stage lookups.

| Existing owner | S32 use |
| --- | --- |
| `prod/pipeline/identity.rs` | Extend its allocation-free canonical SHA-256 writer. Keep S31 palette/resize-plan keys unchanged and compare new keys with the frozen model. |
| `prod/pipeline/preparation.rs::Store` | Extend the existing instance store, shared LRU, capacity ledger, and pinned entries. Preparation and image entries share both caps. |
| `preparation.rs::Call` | Extend its success-only transaction. It already reserves work, borrows preparation, and publishes after the final boundary succeeds. |
| `pipeline/processor.rs` | Retain the single instance, lifecycle state, allocator boundary, and disposal owner. |
| `pipeline/process.rs`, `quantize.rs`, `perturb.rs`, `diffusion.rs` | Connect lookups and ownership transfers around the existing kernel calls. |
| `wasm/processor.rs`, `wasm/quantize.rs` and public package fixtures | Keep current input-copy and durable-result boundaries; exercise actual JS ownership. |

The current materialized candidates are resized RGBA8, perturbed RGBA8, and
indexed output. Process owns resized/optional perturbed/indexed buffers; separable
fused quantization owns perturbed/indexed buffers. Direct quantization and diffusion
own indices. Standalone resize/perturb own their RGBA8 output.

Current matching and field kernels convert pixels directly. They do not materialize
a complete packed f32x3 image. Their Color and Alpha stages remain logical identity
dependencies. Cache a color image only if a delivered path already materializes
one; introducing a color plane merely to cache it would violate the fused-path rule.
Reuse all landed resize, quantizer, field, diffusion-row, and Yliluoma kernels.

## Identity and owned results

Validate each request before lookup, including ignored palette-tail validation.
Copy current input through the existing checked boundary and hash that same owned
snapshot's dimensions and every byte. Hashing precedes image-stage lookup on every
call. Source-copy capacity remains temporary scratch, never a published source entry.
Frontend IDs, caller addresses, calling method, and execution policy never enter keys.

Preserve the frozen framing, recipe version, canonical option order, signed-zero
normalization, f64 alpha thresholds, palette order/duplicates, and truncation flag.
RGBA operation keys differ from output content identities. Hash each newly
materialized RGBA output once; retain its content identity alongside its bytes.
Use that content identity downstream, including when the output came from a hit.
This permits standalone calls receiving returned RGBA8 to share Process stages.
Keep raw Color distinct from Alpha-parented Color; no-dither fused matching and
direct quantization share the same Indexed identity.

RGBA entries own their dimensions, bytes, and output content digest. Indexed entries
own indices plus normalized palette, transparency, and warnings. Their metadata
must survive independent eviction of a prepared-palette entry. Reuse the existing
normalized metadata types and expose a borrowed metadata view to the private
completion adapter, rather than requiring a prepared matcher for an indexed hit.
Return a fresh durable JS copy on every call. Output mutation cannot mutate a cache
entry, and disposal cannot invalidate an earlier public result.

## Capacity and publication

Extend S31's one store and one transaction, not a separately budgeted image cache.
All retained preparation and image entries together obey 128 entries and
`min(256 MiB, memoryLimitBytes / 4)`. Charge actual allocated capacity, including
entry records, metadata, vectors, warnings, pending owners, active scratch, input
copies, and final boundary copies. Count each live allocation once when ownership
moves from work to pending to retained. Caller-owned and returned JS buffers and
fixed module overhead remain outside the limit.

Preflight input snapshot capacity before copying. After hashing and resolving
available hits, preflight the remaining hit-aware execution before any image
kernel runs. Pin hits needed by the call. Release idle scratch before unpinned
LRU entries under total pressure. Oversized results still complete when total
memory permits; they remain uncached. Optional retention must not require an
otherwise unnecessary full-image copy or turn a feasible call into a failure.

Stage only completed, already-materialized buffers. Pending entries remain invisible
to lookup. Reserve retention metadata before final result construction, then make
publication allocation-free after `boundary.complete` succeeds. One success commits
all candidates; validation, allocation, execution, or final-copy failure drops every
pending owner and leaves the instance usable. Earlier LRU hits/evictions need no
rollback. S33 later places the completion callback before this same publication
point; S32 supplies the transaction without inventing callback behavior early.

## Atomic implementation steps

Steps 1 through 4 and the bounded measurement are complete. The report owner files the unmerged stacked PR.

1. [x] **Prove identities.** Extend the existing identity helper and replay frozen
   request-plan fixtures for every method and dither family. Done when production
   keys match the frozen model, including actual RGBA output-content transitions.
2. [x] **Add owned stage entries.** Extend the store/transaction with typed image values,
   shared caps, pinning, capacity transfers, and private hit counters. Done when
   isolated budget, oversized-entry, LRU, and two-entry rollback fixtures pass.
3. [x] **Connect existing pipelines.** Add hit-aware planning and completed-buffer
   publication to all five methods. Done when cross-method calls demonstrably hit
   shared stages and produce identical bytes, palette metadata, and warnings.
4. [x] **Verify public ownership.** Run focused native and installed-package fixtures
   for input/output mutation, final-copy failure, recovery, eviction, and disposal.
   Done when cold/warm equality and retained allocation accounting both pass.
5. [x] **Measure and deliver.** Add hashing/copy-inclusive cold/warm stage-cache cases to
   the delivered benchmark tools. Measure only after coordinator quiescence and
   exclusive-lock clearance. Record actual dependency SHAs, evidence, and a real
   unmerged PR; drain jobs and clean compiler outputs after delivery.

Commit each completed step separately. Keep the literal control baseline and
landed kernels as evidence; optimize only when the prescribed measurements justify it.

## Focused cross-method evidence

- Resize → Process reuses the resized stage. Process → resize returns identical
  RGBA8. Changing source bytes, geometry, filter, support, or anchor invalidates
  the relevant stage while unrelated preparation may still hit.
- Perturb → separable ditherAndQuantize reuses perturbed RGBA8. Quantizing returned
  perturbed bytes shares final Indexed identity. Process → ditherAndQuantize on
  returned resized bytes shares its downstream result, including no-dither,
  separable, diffusion, and Yliluoma families.
- Changed palette order, duplicates, truncation, alpha precision, matching,
  placement, seed, feedback, or dither settings never reuse a mismatched Indexed
  result. Raw/alpha-prepared Color keys stay distinct. Scalar and scheduling
  variants retain identical keys.
- Mutate the same JS input array between calls; mutate a returned result; force
  final-copy failure after at least two pending stages; alternate instances and
  dispose one. Prove correct recomputation, atomic publication, and durable output.
- Mix preparation and image entries until each shared cap evicts. Check LRU refresh,
  scratch-first pressure, oversized-result success without retention, allocation
  failure recovery, and identical cold/warm bytes plus complete metadata.

Benchmarks must separately describe preparation warmth and image-stage warmth.
Use fresh accepted/candidate artifacts and include source copy/hash, downstream
content hashes, final copies, and cross-method sequences. No timing claim belongs
to this preparation checkpoint.

## Preparation handoff

No unresolved product-contract ambiguity was found. The logical Color stages and
the shared preparation/image budget are explicit in the frozen contract. Physical
entry layout and the private borrowed metadata view are implementation choices.
Reconcile them with the final S31 owner after its validated handoff.

The initial preparation commit changed only this plan. The identity checkpoint
adds canonical streaming stage keys and fallible owned image/metadata values.
It changes no pipeline behavior or frozen files. Native validation passes
3 stage-identity tests, 1 preparation-identity test, 16 frozen cache fixtures,
and 1 owned-metadata budget/mutation test. Unwired stage owners currently emit
dead-code warnings; step 2 connects those owners. No measurements ran.

The shared-store checkpoint adds image values to the existing entry enum and
pins up to three image stages in the same call transaction. Record reservations
are fallible. Optional retention returns the materialized owner unchanged when
its cap or reservation prevents caching. Native preparation tests pass 9/9,
including mixed image/preparation caps and LRU, capacity transfer without pixel
copying, two-entry rollback, pinned-hit survival, and oversized-owner return.
Pipelines still use preparation-only behavior until step 3.

The pipeline checkpoint snapshots and hashes current input before image lookup.
Resize and perturb reuse their RGBA stages. Process, quantize, and fused calls
share one indexed-stage coordinator around the unchanged kernels. Native tests
prove cross-method hits for all four families, including after matcher eviction.
The completion adapter now borrows normalized metadata independently of preparation.

Production budget fixtures now locate the mandatory cold execution limit with
fresh-instance capacity searches. One byte below that limit copies only the
input snapshot and runs no image kernel or completion. Optional retention may
succeed at a lower capacity than the full retained cold peak. Physical allocator
fixtures still prove release after every mandatory allocation failure. A new
fixture injects optional image-record failure and proves successful durable output
and full release on disposal. The diffusion-row witness observes deallocation
before replacement; image eviction may reduce its live-byte total further.

Full native `cargo test --locked --tests` passes after the corrected parent join.
The 14 processor tests include both physical parent witnesses and optional-record
failure. A further native hit fixture proves normalized palette and both warnings
survive matcher eviction, returned metadata mutation, and disposal. Public fixture
`8bd9208138322776433cb0eb3e9dcf0f4d15de52` is joined. Source-only staging (2)
and scalar glue (3) tests pass. Interface tests await generated scalar declarations.
Final artifact builds wait for the coordinator's benchmark-protocol review clearance.

A focused mixed-entry pressure witness reproduced lookup recency reversal during
publication. Existing hits now retain their `take()` timestamp; only new entries
receive publication timestamps. The witness passes for success and failure paths.
All 25 native library tests pass after this fix. The coordinator cleared the
benchmark protocol at `d51a70a2daf054357d16ea66b235da3733c02888` for the final join.

That protocol is joined. Its native complete-call adapter now borrows the same
normalized result metadata as the Wasm adapter. The declared stage matrix passes
all 5 untimed native/frozen composition tests, and all 5 focused Node protocol
tests pass. Candidate preparation uses the official native/public preparers once
this clean joined revision is validated. The coordinator owns the trusted guard,
quiet clearance, actual paired measurements, and final performance decision.

Publication pressure also compares an incoming prior hit with stored victims.
A red/green fixture pins a 16,000-byte image, prepares a newer palette, and sets
the quarter cap below their sum while both individually fit. The older incoming
image now drops instead of evicting the newer palette. All 26 native library
tests pass. The complete native `--tests` suite passed at the preceding joined
revision; final focused validation covers this publication-only correction.

## Candidate artifacts and installed conformance

Both official preparers built clean runtime revision
`d638f87c3a16824ef52964bbb611ef91b173f2ee`. Native binaries and provenance remain
in `target/s32-candidate-d638f87c-native`. The installed public bundle and provenance
remain in `target/s32-candidate-d638f87c-public`.
Its `ditherette.tgz` SHA-256 is
`8a51e04d08cfdf73bd022ccef1167fed36c74f8267ea1615e19e46df7636fc80`.
Compiler targets are the worktree-local paths declared above. No measurements ran.

Test-only delivery head `dbf739595724d7ac6d963cc85123d2c211b68b7d` joins the corrected
external-tarball driver and updates obsolete preflight-copy assertions. Native/private
candidate fixtures require exactly one snapshot before remaining-budget rejection.
Cross-revision public fixtures allow zero or one only for total-budget failures.
Malformed requests and input-only budget failures retain their exact zero-copy checks.
Error code/path, no returned output, and recovery assertions remain intact.
Runtime, package implementation, and frozen files are unchanged after the artifact revision.

Final conformance passes interface tests (34), private Wasm ABI tests (16), and
focused installed stage-ownership tests (4). The broad installed suite also passes
all 4 tests against the exact candidate tarball and its prepared frozen oracle.
Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 each pass 367 frozen
Yliluoma vectors and 734 untimed actual benchmark-adapter calls. The semantic input
fixture remains at `target/s32-d638f87c-yliluoma-fixtures.json`.

Browser session 76291 exited successfully. All candidate build/test jobs are drained.
Retain artifacts and compiler outputs for the coordinator's exclusive trial.
Further builds, tests, edits, and reviews wait for its measurement-complete handoff.

## Measured delivery

The coordinator completed the exclusive trial and transferred report/PR ownership after every worker exited.
The [measurement report](73-benchmark-results.md) and [raw-value summary](73-benchmark-results.json) bind accepted
`d51a70a2daf054357d16ea66b235da3733c02888` to candidate `d638f87c3a16824ef52964bbb611ef91b173f2ee`.
The audit verifies 128 starts/reaps, maximum one live worker, 2,560 samples, and 64 exact role pairs.
All four overall performance gates regress. Cold Lanczos3 on all four runtimes and cold Process on all browsers remain S41 release blockers.
Five individual cases remain inconclusive. No extra tuning, retry, candidate promotion, or release-performance pass is claimed.
S32 delivers required cache functionality with its measured costs; those costs remain release blockers rather than hidden acceptance exceptions.

The PR stacks on `impl/v1-s31-preparation` at `59b1fe3acdbeae27bbb8ab780b46d2b6b9d67a76`.
The required ancestry-preserving rebase leaves conformance head `59b5a004c98c4cff255b30f51025d8ec8143786d` unchanged.
Report-only changes follow that head. Measured runtime and all retained artifacts stay unchanged.
The real non-draft [PR 122](https://github.com/mia-cx/ditherette/pull/122) is open and remains unmerged.
Report commit `cbd0003c8e0ef4bc1f6a751280bcee4a9ee8596f` contains the audited evidence summary.
After the real unmerged PR is filed, return the six worktree-local compiler targets in S32 stages/benchmark to the coordinator.
Copied binaries, native/public artifacts, oracle evidence, prepared snapshots, and result folders stay retained.
The coordinator owns S41 issue recording and audited compiler cleanup; the report owner performs neither.
