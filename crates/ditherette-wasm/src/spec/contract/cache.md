# Cache and memory control reference

Read this reference when implementing instance memory, preparation reuse, image-stage caching, or callback publication.
[Decision 37](https://github.com/mia-cx/ditherette/issues/37#issuecomment-5562808049) owns the policy.
[Decision 24](https://github.com/mia-cx/ditherette/issues/24#issuecomment-5562864637) owns callback ordering.
`cache.rs` supplies the readable ownership model; `lifecycle.rs` supplies call and callback guards.

## What the model represents

`CacheModel` owns small byte values in a plain oldest-first vector.
Each value represents an already-materialized stage and carries its declared allocated capacity.
That capacity represents the production allocation, including its associated owned metadata, rather than logical image length.

`MemoryPlan` separates planned work, active scratch, and transient boundary copies.
Work includes materialized cache candidates until ownership moves into pending entries.
The capacity ledger separately counts retained entries, idle scratch, active scratch, work, copies, and pending entries.
Every live represented owner counts once.

These are state-model capacities, not measurements of the model's actual Rust vectors or hashing allocations.
Tiny values and capacity numbers allow exact pressure/failure fixtures without allocating large images.
This module does not make spec kernels fallibly allocate, estimate their physical peak, or enforce real Wasm memory limits.
Production must supply complete capacity plans and connect actual allocation failure at its allocation boundary.
Caller-owned and returned JS buffers and fixed module overhead remain outside the approved memory limit.

## Identity

`source_identity` hashes the versioned RGBA8 prefix, both little-endian dimensions, and every current input byte with SHA-256.
It keeps only the 256-bit result. Original source buffers are never retained by this operation.
Callers validate storage before hashing and recompute the identity when beginning another call.
The hash framing matches the existing benchmark input-identity precedent without importing benchmark code.

`palette_identity` hashes the retained ordered prefix and whether truncation occurred.
Duplicates and Transparent positions remain distinct.
An input of 256 entries differs from 257 identical-prefix entries because the latter emits a truncation warning.
Once truncation is true, differences after entry 256 have no semantic effect.

`stage_identity` hashes its parent, recipe version, and typed `StageOptions`.
Source-independent palette/plan preparation uses no image parent.
Resize-plan options contain source geometry because plans depend on dimensions rather than source bytes.
The stage tag prevents different semantic stages from sharing a key.
The calling public method, thread count, callback, frontend source ID, and scheduling options are absent.
For example, process and resize use the same Resize stage when source and output settings match.
Quantize and fused no-dither matching both use Indexed with `DitherPolicy::None {}`.

An operation key and an RGBA8 output content identity are different values.
Resize and Perturb operation keys use their input content identity and normalized settings.
After materializing their output, hash its dimensions and bytes once with `source_identity`.
Downstream operations use that output content identity, matching standalone calls receiving the same returned RGBA8.
`CachedValue.rgba_content` retains the output identity alongside owned stage bytes, so hits need no output rehash.

`request_identity_plan` validates and describes every Request variant without executing image kernels.
Its ordered stages use Input, earlier Operation, or earlier RgbaOutput parents.
`key_at` resolves an available prefix for cache lookup before running its operation.
`resolve` supplies all operation keys once required RGBA output identities are available.
The final identity names the result's operation, not its output content.
Callers associate each supplied content digest with its actual materialized output or cached output metadata.

Alpha is a typed logical stage containing palette identity and AlphaPolicy.
Indexed matching's Color stage uses the Alpha operation key as its parent.
It therefore cannot alias raw Color or differently alpha-prepared Color.
This logical stage does not require an additional materialized alpha buffer.
The canonical plan composes process as Resize then fused matching.
Separable fused matching composes raw Color, Perturb, then Alpha, Color, and no-dither Indexed at the RGBA8 boundary.
Direct quantize uses the same final Alpha/Color/Indexed composition.

Settings must already satisfy request validation.
Canonical JSON orders object keys, and signed floating zero normalizes to positive zero.
The f64 alpha threshold retains its precision; 127.9999999 differs from 128.
Both diffusion feedback tags, matching policy, palette identity, and every dither option remain part of indexed identity.
Changing a retained palette or alpha policy therefore invalidates affected prepared and indexed values.

The model retains only entries supplied through `stage`.
Fused kernels may supply intermediates they actually materialize; keys do not create imaginary intermediate buffers.
Supplying original source buffers as cache values is outside this contract.

## Memory and eviction

`begin` first checks the call's required capacity against the instance limit.
Checked addition maps an overflowing plan to `memory-limit` at `memoryLimitBytes`.
No processing begins for an impossible plan.

For a feasible plan, pressure removes idle scratch before least-recently-used entries.
Cache hits move an entry to the newest position.
The retained-entry cap remains 128 entries and `min(256 MiB, memoryLimitBytes / 4)`.
Cache-cap pressure evicts oldest entries; dropping scratch cannot lower retained-entry bytes or entry count.
An entry larger than the cache byte budget stays uncached while the otherwise feasible call succeeds.

Scratch reuse is an explicit execution choice, not an identity field.
When requested, `begin` transfers all idle scratch capacity into active ownership.
The ledger counts the full retained capacity even if the request needs fewer bytes.
Without reuse, new active scratch and old idle scratch are separate owners.
Successful active scratch becomes idle; failed active scratch is released.
This aggregate model specifies ownership and budgets, not contiguous allocation suitability or a scratch-pool implementation.

`stage` transfers a materialized value's capacity from work into pending ownership.
It rejects an impossible transfer as a model-control error.
An oversized or already-published/staged identity returns false and does not create another pending owner.

## Call integration

Pair one CacheModel with one InstanceModel for its entire lifetime.

1. Start the lifecycle call, then call `cache.begin(plan, reuse_idle_scratch)` before processing.
   If preflight fails, end the lifecycle call through `InstanceModel::fail`.
2. Read published entries with `lookup`; stage completed candidate values with `stage`.
   Pending entries remain invisible to lookup.
3. Mark final output ready on InstanceModel and report the enabled completion callback.
   After that callback succeeds, call `cache.finish(&mut instance)`.
   This method invokes `InstanceModel::finish` before publishing anything.
4. On expected processing failure, call `cache.fail` and `InstanceModel::fail`.
   On a thrown callback, call `InstanceModel::callback_failed` and `cache.fail`.
   On unexpected browser allocation failure, `cache.allocation_failed` drops the cache call and returns
   `wasm-memory-unavailable` at `memory`; end the lifecycle call separately.
5. Call `cache.dispose(&mut instance)` to release represented ownership.
   Active-call disposal rejects reentry; repeated successful disposal is harmless.

Failures discard all pending entries and publish no partial result.
They need not undo previous cache hits or eviction of old entries.
The instance remains usable after an expected failure.
Lookup returns an independent small byte copy, allowing tests to model durable returned ownership.
Disposal clears retained values and scratch but never invalidates those copies.
It makes no guarantee that Wasm pages shrink before module collection.

## Evidence

`tests/spec_contract_cache.rs` uses an independently calculated SHA-256 vector and tiny capacity ledgers.
It also executes naive resize/perturb outputs to verify actual cross-method key equality at RGBA8 boundaries.
Every Request variant and all four dither families appear in those identity-composition fixtures.
It covers source mutation, dimensions, palette order/duplicates/truncation warnings, normalized zero,
f64 threshold precision, feedback identity, stage separation, exact budget boundaries, overflow,
scratch-first eviction, scratch reuse, LRU hits, both cache caps, oversized results, two-entry callback rollback,
successful completion, allocation failure recovery, isolated instances, durable copies, and disposal.

Actual image processing, real allocation preflight, and browser callback plumbing remain separate integration checks.
Those implementations must follow this frozen model; their data structures and allocation strategies may differ.
