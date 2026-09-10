# Literal cache control baseline

`cache.rs` copies the frozen state model byte-for-byte. Relative contract imports use existing production contracts.
Read [the frozen allocation boundary](../../spec/contract/cache.md) before integrating real ownership.
The tiny modeled values and declared capacities do not enforce physical allocation limits.
S31 integrates preparation and scratch only. Image-stage caching remains S32 work.

## Approved source reuse exception

On 2026-09-10, Mia approved exact-byte verified reuse instead of mandatory per-call source hashing.
The frozen specification remains unchanged as the reference implementation.

Production retains the last successful source's initialized scratch bytes, dimensions, and dependency revision.
Before reuse, the private Wasm boundary validates the current input and compares every byte against that owned snapshot.
Equal bytes and dimensions reuse the revision. A mismatch copies the input and assigns a new revision.
Object identity never establishes equality. Views use their own bytes, including their actual offset.

The snapshot uses the existing budgeted source buffer, not a second image allocation.
Memory pressure can discard it. Failed calls and disposal discard source bytes and identity together.
Successful result construction and completion callbacks must finish before publication.
Returned outputs remain independent copies.

## Approved dependency caching exception

On 2026-09-10, Mia approved staged dependency caching instead of hashing image buffers.
The frozen cache model remains an unchanged reference, not the runtime caching policy.

Source revisions belong to the processor and are never recycled, even after failure or scratch eviction.
Resize keys depend on the source revision and resize settings. Perturb keys depend on the resize key and field settings.
Indexed keys depend on the preceding image key, palette, alpha, matching and dither settings.
Changing an upstream dependency makes its downstream entries ineligible. Unaffected preparation remains reusable.
The shared bounded LRU may retain old branches until eviction. A final hit skips all intermediate lookups and execution.

No source or intermediate RGBA bytes are hashed, including on cold calls.
Only the existing small canonical settings and palette records still use SHA-256 as compact lookup keys.
Identity resize passes the owned source through internally. Public resize still returns an independent copy.
If a public staged call supplies a retained RGBA result as input, exact bytes and dimensions verify its existing dependency key.
Otherwise an input change starts a new revision. Different recipes no longer deduplicate merely because their output pixels match.

Native boundaries that omit exact-byte verification remain correct but start new source revisions on each copied input.
The native benchmark boundaries implement the same comparison contract as the browser adapter.
