# Literal cache control baseline

`cache.rs` copies the frozen state model byte-for-byte. Relative contract imports use existing production contracts.
Read [the frozen allocation boundary](../../spec/contract/cache.md) before integrating real ownership.
The tiny modeled values and declared capacities do not enforce physical allocation limits.
S31 integrates preparation and scratch only. Image-stage caching remains S32 work.

## Approved source reuse exception

On 2026-09-10, Mia approved exact-byte verified reuse instead of mandatory per-call source hashing.
The frozen specification remains unchanged as the reference implementation.

Production retains the last successful source's initialized scratch bytes, dimensions, and SHA-256 identity.
Before reuse, the private Wasm boundary validates the current input and compares every byte against that owned snapshot.
Equal bytes and dimensions reuse the identity. Any mismatch copies the input and computes its identity again.
Object identity never establishes equality. Views use their own bytes, including their actual offset.

The snapshot uses the existing budgeted source buffer, not a second image allocation.
Memory pressure can discard it. Failed calls and disposal discard source bytes and identity together.
Successful result construction and completion callbacks must finish before publication.
Returned outputs remain independent copies.

The first call still hashes its source. Verified reuse still scans all input bytes.
This exception removes repeated hashing, not every browser or boundary cost.
