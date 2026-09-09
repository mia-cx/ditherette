# S41 capped-output transport candidate

## Scope

Branch `impl/v1-s41-capped-transport` starts at S40 CI
`95706738e4f3179824a66c80bb5728ce97a34b4b`. This benchmark-only candidate
addresses the required worst-case capped output from issues #83, #38, and #2.
It does not change the public package, runtime, frozen semantics, or selected
production candidate.

Owned files are `scripts/benchmark-public-timing.mjs`, its tests, and this plan.
S37 owns the release matrix. Any serialization or case-schema changes need
coordinator approval and explicit ownership first.

## TODOs

- [x] Reproduce the retention refusal without allocating image data; inspect
  production preflight and transport bounds.
- [x] Add an explicit bounded retention override and focused collector tests.
- [x] Add the approved indexed-only wire envelope and tiny decoder tests.
- [x] Record required wiring, transport blockers, and a quiet-window resource plan.

## Findings

The required case is `capped-indexed-8192x8192`. It runs Process from a 1x1
source with centered nearest resize, no dither, sRGB matching, and 15 visible
palette entries plus transparency. S37 owns the exact settings.

`indexed::run` prepares four byte buffers. This case needs 4 source bytes,
268,435,456 resized RGBA bytes, no perturbed buffer, and 67,108,864 index bytes.
Nearest's exact integer upscale needs no axis maps. Palette and owner records
are small. The package's default 1.5 GiB logical budget supports this shape;
there is no full-image floating-point color plane. This is a code inspection,
not a full-size allocation check.

The existing collector reserves one batch slot and four stability evidence
slots. An indexed result reserves its maximum 1,024 palette bytes and 1,024
bookkeeping bytes. Five slots need 335,554,560 bytes. The normal 64 MiB limit
rejects this before the first call. A per-case 384 MiB limit would admit one
call but reject a two-call batch. Default behavior must remain unchanged.

The allocation-free reproduction calls
`retainedOutputSlots(1, 8192 * 8192 + 1024, 384 * 1024 * 1024)`.
On the base revision it throws the existing 64 MiB budget error.

Serialization remains separate. `verificationOutput` expands typed indices
into number arrays. The frozen oracle also returns JSON arrays. The loopback
server parses full JSON, and Node stringifies the combined reference and actual
evidence. Three indexed outputs with values 0..15 can require 603,979,779 JSON
characters, excluding metadata. This exceeds Node's 536,870,888-character
string limit. Exact uniform output may serialize smaller, but instability must
still preserve both actual outputs. Raising retention alone cannot guarantee
the capped case's evidence path.

Root approved a per-output benchmark-only envelope. It retains normal metadata
with empty transit indices and carries the exact indices as lowercase hex.
Three maximum-area hex outputs need 384 MiB of ASCII before small metadata,
below Node's string limit. Rust decodes the envelope before the existing exact
verifier. The frozen oracle's serializer and shared benchmark API stay unchanged.
The JavaScript oracle wrapper compacts its already-returned output. S37 owns
case metadata and public-page forwarding. This agent owns the new wire helper,
oracle wrapper, private Rust decoder, and asset registration.

The initial collector checkpoint passes 13 focused Node tests. Both new tests
failed on the base's 64 MiB refusal first. Mock calls allocate no image bytes.

The complete wire implementation preserves exact bytes in an
`indexed8-hex-v1` envelope. Its metadata retains an empty transit index array.
The separate `indices_hex` string uses canonical lowercase hex. Decoders check
dimensions, palette indices, transparency, warning bounds, encoding, and length.
Rust restores the existing `VerificationOutput` before verification. Undeclared
ordinary cases keep their existing parser and 64 MiB collector limit.

The compact HTTP bound is two actual hex outputs plus existing metadata/sample
allowances. The complete Node response adds the independent reference, for
three hex outputs plus 1 MiB and sample allowances. Priming is excluded from
capped-wire cases because a fourth maximum-area hex output would exceed Node's
string limit after metadata. S37 owns this validation and page integration.

## Quiet-window resource check

1. Join S37's final page/schema checkpoint and this transport candidate. Build
   fresh hash-bound scripts and oracle provenance. Preserve the frozen Wasm
   serializer and public package semantics. Use the actual capped matrix entry,
   a scalar package, default package memory budget, and explicit 384 MiB retention.
2. Drain implementation jobs. Root checks host headroom and runs one isolated
   pre-timing resource check. Keep the required case held until its oracle,
   Process/staged comparison, exact output transport, and process shutdown finish.
3. Retain peak RSS and exit status. Do not treat the collector budget as a host
   memory guarantee. If allocation fails, retain the error and report the capped
   lane incomplete. Do not substitute a smaller case or raise the default limit.

Known live storage includes about 320 MiB package scratch, at most 320 MiB plus
small metadata in collector/evidence slots, and 64 MiB typed reference indices.
Encoding one output temporarily holds up to two 128 MiB hex representations.
Node retains up to three 128 MiB hex strings and one combined JSON string.
Rust holds the incoming line, its parsed strings, and decoded byte vectors.
These phases are sequential but garbage collection and Wasm high-water memory
can retain earlier allocations. The frozen oracle still constructs its original
JSON number array once; its own allocation peak has not been measured. The
initial native reference JSON also remains on the existing transport input path.
The host's 5.7 GiB available RAM therefore requires the held real resource check.

## Focused results

- Node 24.19.0: 37 passed, one existing oracle-policy test skipped. Files were
  `benchmark-indexed-wire.test.mjs`, `benchmark-public-timing.test.mjs`,
  `benchmark-public-browser.test.mjs`, and `prepare-benchmark-oracle.test.mjs`.
- Native Rust: four private wire tests, 11 browser-worker tests, and ten asset
  tests passed. All used this worktree's `target/compiler`.
- No full-size pixel allocation, actual browser, Wasm build, or measurement ran.
- The new helper enters public script staging, host/conformance fixture staging,
  and oracle wrapper source provenance. Ordinary served entrypoints stay intact.

## Validation and holds

No full-size probe, browser, Wasm build, or benchmark runs in this task.
Root reserves a quiet window after all implementation jobs exit. The host has
7.6 GiB total RAM and about 5.7 GiB available; logical package capacity is not
a bound on browser/Node JSON copies, oracle work, or Wasm high-water memory.
