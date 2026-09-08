# Developer row-band benchmark artifacts

Refs #78 and #79. Root owns host-worker runtime wiring and measurements.

## Completed

- [x] Add explicit `build.mjs scalar|threads --bench-subjects`. Keep normal
  compiler, profile, linker flags, wrapper generation, and output paths.
- [x] Add `prepare-public-benchmark.mjs NEW_DIRECTORY --bench-subjects`.
  Rebuild both variants, stage the same wrapper, and verify developer markers
  plus the private policy export before packing.
- [x] Record typed optional `build_mode` provenance. Preserve schema-one records,
  strict unknown-field rejection, fixed revision checks, source digests, and
  complete artifact inventories.
- [x] Validate six Node preparation tests and ten native browser asset tests.

## Handoff

Developer variant directories contain `build-mode.json` with schema 1, mode
`bench-subjects`, and their scalar/threads variant. Normal artifacts have no
marker or private policy export. Provenance writes `build_mode` as `public` or
`bench-subjects`; old records can omit it.

Root supplies `scripts/benchmark-row-policy.mjs`, already listed for snapshot
copying. Join that file and the existing private Wasm policy export before
preparation. No public exports, processing methods, or frozen guards changed.
No Wasm builds or measurements ran. Tests used Node 24.19.0 and this worktree's
fresh native `target/compiler`; all jobs exited.
