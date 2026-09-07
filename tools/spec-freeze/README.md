# Frozen reference

`checkpoint.json` names the complete S17 reference at commit
`cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`. Its content identity survives rebases.
The checkpoint is recorded, never recalculated from a PR parent.
The retention branch `reference/ditherette-v1` keeps that named commit reachable
through implementation-stack rebases. Its mutable tip is not a source of trust;
validation uses the recorded full SHA and content digest.

## Shared dependency audit

The frozen closure includes every file under `src/spec` and `src/image` in
`crates/ditherette-wasm`, plus `examples/generate_blue_noise.rs`.

| Shared image files                          | Observable behavior protected                                                 |
| ------------------------------------------- | ----------------------------------------------------------------------------- |
| `dimensions.rs`, `stride.rs`, `validate.rs` | Dimension validation, overflow handling, exact buffer lengths, row addressing |
| `formats.rs`, `pixel.rs`                    | Channel counts, indices, storage types and defaults                           |
| `view.rs`, `owned.rs`                       | Pixel access, initialized allocations, strides, owned results                 |
| `contracts.rs`                              | Palette parsing, transparency, warning codes, result metadata                 |
| `mod.rs`, `rgba8.rs`                        | Shared exports and packed RGBA layout helpers                                 |

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
The record also binds every resolved procedural-macro implementation and its
dependencies, including Wasm binding expansion. Attribute names alone do not
identify the macro behind an imported alias.

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
Root reexports accept only their exact audited feature gates and literal doc
strings, so procedural attributes cannot inject code outside isolated roots.
Semantic modules cannot declare foreign blocks or symbol-linking attributes.
Type-checking alone cannot prove where an external symbol resolves. Wasm adapter
imports remain allowed at the boundary, including `wasm-bindgen` extern blocks.
Explicit `no_mangle`, `export_name`, `link_name`, and `link` attributes are forbidden throughout source,
including adapters, because they can interpose native arithmetic symbols.
Raw identifier spellings receive the same checks. Wasm binding imports need none
of these explicit symbol attributes.
Adapters accept audited procedural attributes and derives; new expanders require
policy review. `wasm.rs` and its submodules may call production, never spec or
benchmark subjects. Benchmark adapters retain their two-family comparison role.
Macros that generate attributes require policy review because they can hide a
module path behind substituted tokens.

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

## CI trust and bootstrap

The read-only `pull_request` workflow checks out the exact base SHA separately.
It executes the base's guard and reads the base's checkpoint against candidate
files. That guard rejects candidate changes to its executable policy, checkpoint,
dependency record, tests, or workflow. A candidate cannot bless changed spec bytes
by updating hashes or moving its parent. Audit prose may change independently.

S18 itself has no base policy. Its one bootstrap exception requires the exact
validated S17 base SHA and reports that human review is required. It runs the new
policy as a test, not as an independent proof that the policy is trustworthy.
Subsequent bases missing the policy fail closed. No privileged
`pull_request_target` job executes candidate code.

Repository administrators must require the `Frozen reference` check and protect
workflow/policy changes. An ordinary PR workflow cannot stop its own deletion or
replacement with a same-named no-op check. The trusted-base comparison detects
such edits only when the trusted checker actually runs. This implementation does
not configure repository rules, activate default-branch workflows, or claim that
an unmerged stack has those protections enabled.

To validate a candidate with a trusted checkout locally:

```sh
node /absolute/trusted/tools/spec-freeze/guard.mjs --root /absolute/candidate --trusted-root /absolute/trusted
```

The checker uses installed Rust 1.97.0 and rejects host/repository Cargo config
files that could redirect its compiler. Cargo's downloaded registry and compiler
binaries remain trusted build infrastructure. A matching source checkpoint is
not a claim about an arbitrary binary built elsewhere with different flags.
Fresh conformance/performance artifacts must retain their actual build identity.

Run `node --test tools/spec-freeze/guard.test.mjs` to exercise temporary mutations.
Each fixture cleans up in `finally` and rechecks the original frozen content.
The JSON feature fixture updates only temporary lockfiles and may download its
additional registry dependencies. It never runs a benchmark.
