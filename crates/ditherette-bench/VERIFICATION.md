# Three-way verification

`ditherette_bench::verification` compares reference, accepted production, and
candidate outputs. It shares the RGBA comparison engine used by existing resize
commands. Verification fixtures produce no performance evidence. One controlled
one-pixel fixture exercises the measured-invocation path to reject stale
verification.

`ditherette_bench_api::verification` owns generic `VerificationCase<P>` and
`VerificationSubject<P>`. `P` is the implementation's concrete request type,
including borrowed S03 requests. There is no second request/settings schema.

## Case and result identity

Each case records its operation, named recipe/version, working space, output
dimensions, source digest, and normalized-settings digest. Source hashing includes
dimensions and every RGBA byte. Settings hashing accepts typed serializable
settings and canonicalizes object key order. Validate settings through the
concrete request contract before hashing; include palette order where relevant.

Each role records its subject ID, full source revision, and SHA-256 artifact
content digest. `Digest256` serializes all 32 bytes. Callers obtain identities
from the actual prepared artifacts. Historical labels or abbreviated revisions
cannot establish a required comparison.

The output model covers RGBA8, indexed pixels, and packed three-coordinate f32
color data with separate byte alpha. Indexed output retains the normalized RGBA
palette, transparent index, warning codes/messages, and warning order.
Color artifacts serialize exact coordinate bits, including invalid non-finite
values. Comparison rejects non-finite coordinates and reports numeric coordinate
maximum, mean, and RMS error separately from RGBA color distances.

## Outcomes and review evidence

All three role slots are required. Wrong identities, invalid storage, or missing
outputs produce `incomplete`. Palette, transparency, warnings, alpha, and output
representation remain hard requirements. Exact index comparison remains visible
even when duplicate palette entries render identically.

`exact` requires all three pair comparisons to agree. A pre-freeze exact result
is provisional; `release_conformant()` also requires a frozen reference.
Numeric RGBA bounds are diagnostic. A bounded non-exact result needs Mia's visual
approval and cannot pass automatically. Coordinate errors do not use byte-RGBA
bounds. This verifier provides no implicit approval override.

Use `verify_and_preserve` for runs that need retained evidence. On every non-exact
or incomplete outcome it writes `results.json` to a fresh directory. Raw results
survive even when invalid storage cannot render. Available valid outputs produce
reference, accepted, and candidate PNGs plus all three pairwise difference PNGs.
Differences are opaque; alpha error contributes to all three visible channels.
Existing bundles cannot be overwritten. Missing parent directories are created
before the fresh final bundle directory is reserved.

`verify_three_way` is the pure comparison interface for fixtures or coordinators
that retain the complete returned evidence themselves. Missing or invalid outputs
must remain in the report; they are never omitted to create a passing subset.

Legacy resize reports now distinguish exact `passed` from `within_bounds`.
Accepted-baseline writes and indexed refreshes require complete exact proof for
every result before replacing any file. `--allow-correctness-failures` permits
diagnostic runs but cannot promote their failing results. Historical artifacts
without the new exact proof cannot be promoted through this path. Native
measurement results retain a full SHA-256 digest of the final output. Exact
verification records bind the checked candidate bytes to that digest. A changed
measured output, missing digest, or mismatched digest cannot become an accepted
baseline. The legacy display checksum remains unchanged. Tiling sweeps use
numeric bounds for diagnostic continuation; they do not grant accepted-baseline
approval.

## S17 adapter handoff

1. Use concrete completed request types as `P`. Bind reference, accepted, and
   candidate adapters to the actual artifact identities and shared case identity.
2. Reuse `bench_subjects::verification::indexed_output` and `rgba_output` for
   public storage. `reference_resize` and `production_resize` already call the
   existing resize registry. An unavailable production mode returns an error.
3. Add color inverse rendering only through the actual completed inverse recipe.
   Until then, packed coordinate comparisons retain raw data without invented
   RGBA previews. Keep each result's explicit working-space identity.
4. Set `ReferenceState::Frozen` only after checking the named reference commit
   and frozen content checkpoint against the tested reference artifact. Run all
   required cases through `verify_and_preserve`; retain every failed bundle.

These adapters are for conformance. They do not replace the existing direct
resize measurement calls or add production kernels. Fresh paired timing and
artifact coordination belong to S06 and follow [exclusive execution](EXECUTION.md).
