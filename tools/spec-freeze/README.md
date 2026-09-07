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

## Validation boundary

Run `node tools/spec-freeze/guard.mjs` for a local check. It verifies content,
compiler, actual core/benchmark dependency resolution, Rust syntax, and independent
native/Wasm compilation. The syntax check visits inactive `cfg` branches too.
The reference and production compile separately with their own modules plus the
frozen image tree. Production remains pure Rust; Wasm/JS adapters stay in `wasm`.
Neither semantic family may call back through `wasm` or `bench_subjects`.

Source redirection (`#[path]`, `include*`), environment-based source injection,
and escaping macro definitions (`macro_export`, `macro_use`) are forbidden.
Local macros and ordinary optimization attributes remain available. Rust, not a
custom resolver, handles imported aliases and helper dependencies. The real crate
root retains its audited module set and permits explicit adapter reexports.

The two real Cargo manifests retain their approved profiles. Core release uses
`opt-level = "s"`; benchmark release uses Cargo's defaults. Inherited debug
overflow checks and release wrapping remain distinct, as specified by the legacy
diagnostic adapters. Candidate flags, wrappers, build scripts, patches, or Cargo
configuration cannot become hidden inputs to the validation compiler.

The guard policy is separate from the frozen reference. A new production
dependency, module-root arrangement, or source-generation mechanism may require a
policy extension. Review that extension explicitly against a trusted policy
checkout while keeping the checkpoint bytes unchanged. It cannot authorize a
reference edit or select a replacement digest through an ordinary candidate PR.
