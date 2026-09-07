# Frozen reference

`checkpoint.json` names the complete S17 reference at commit
`cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`. Its content identity survives rebases.
The checkpoint is recorded, never recalculated from a PR parent.

## Shared dependency audit

The frozen closure includes every file under `src/spec` and `src/image` in
`crates/ditherette-wasm`, plus `examples/generate_blue_noise.rs`.

| Shared image files | Observable behavior protected |
|---|---|
| `dimensions.rs`, `stride.rs`, `validate.rs` | Dimension validation, overflow handling, exact buffer lengths, row addressing |
| `formats.rs`, `pixel.rs` | Channel counts, indices, storage types and defaults |
| `view.rs`, `owned.rs` | Pixel access, initialized allocations, strides, owned results |
| `contracts.rs` | Palette parsing, transparency, warning codes, result metadata |
| `mod.rs`, `rgba8.rs` | Shared exports and packed RGBA layout helpers |

These files contain storage behavior, not color or matching algorithms. Both
families may use them, but neither may add semantic code beneath a shared helper.
Freezing the small image tree prevents an apparently infrastructural edit from
changing oracle results. New production storage belongs outside this frozen tree.

The full mode, kernel, control, adapter, and export inventory is frozen at
`src/spec/contract/inventory.md`. Blue-noise provenance includes its readable
generator, launcher, rank tile, analysis JSON, and specification.

## Reference build inputs

Reference code uses standard Rust, Serde derives, `serde_json`, and SHA-256 from
`sha2`. The cache identity in `spec/contract/cache.rs` relies on JSON object keys
being sorted. Enabling `serde_json/preserve_order` changes that identity without
editing a reference file. Validation must compare resolved features as well as
versions and registry checksums, including transitive dependencies.

Reference validation uses Rust 1.97.0, matching the S02 package branch. The
inherited root toolchain still says `stable`; that floating selector is not the
reference compiler identity. Production packaging may adopt the S02 pin without
changing frozen content. The S06 benchmark provenance build script is unrelated
to reference compilation and remains allowed.

## Conformance identity

`checkpoint.json` exposes an `identity` record with `state: "frozen"`, the full
reference revision, and the content artifact digest. A conformance client may
use S05 `ReferenceState::Frozen` only after validating this checkout against that
record. Registration alone does not prove a checkout is frozen. Existing
pre-freeze/control experiments remain pre-freeze; this does not promote them or
any production candidate.
