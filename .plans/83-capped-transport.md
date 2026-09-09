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
- [ ] Add an explicit bounded retention override and focused collector tests.
- [ ] Record required wiring, transport blockers, and a quiet-window resource plan.

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

## Validation and holds

No full-size probe, browser, Wasm build, or benchmark runs in this task.
Root reserves a quiet window after all implementation jobs exit. The host has
7.6 GiB total RAM and about 5.7 GiB available; logical package capacity is not
a bound on browser/Node JSON copies, oracle work, or Wasm high-water memory.
