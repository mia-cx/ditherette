# S24 quantize benchmark integration

Start at coordinator checkpoint `ebbc1c5a32a77f68c2e59595e84a7b227c25c7ee`.
The authorized join adds native quantize `f83e58a46426ac2927ff481b78219d8163a06b05`.
The literal implementation baseline is `a23260edec0452fd17c13073636f548b07804230`.

## Ownership

Own benchmark protocol, timing, verification adapters, registry fragments, and focused fixtures.
S01 owns the public quantize package and ABI. The coordinator owns integration and measurements.
Prod, spec, image, freeze policy, and public package files stay unchanged in this worktree.

## TODOs

- [x] Extend typed native/public operation identity and indexed transport validation using existing request/output models; verify without timing.
- [ ] Register literal-baseline and prepared quantize adapters, reusing the native timing loop and exact three-way verifier.
- [ ] Declare the bounded matrix, validate protocol/adapter fixtures, commit/push clean checkpoints, and drain.

## Interface

Quantize settings retain ordered palette entries, the full alpha policy, and the exact matching tag.
Use existing contract types and S05 `VerificationOutput::Indexed8`; preserve palette RGBA, transparent index, warning codes, and warning messages.
The browser maps those settings directly into the actual package `quantize` request.
Preflight and final comparison serialize outputs outside timing. No benchmark-only hashing enters the public call.
Package cache capability remains none. Fresh/primed instances are distinct from application-cache state.

Native complete quantization includes validation, preparation, allocation, conversion, matching, and result ownership.
Source bytes are borrowed; this is not the public JavaScript call or its input/result copy cost.
Use a separate native-complete-call scope. Existing resize evidence remains readable unchanged.
Refactor the existing native loop around a callable workload while retaining warmup, calibration, samples, and observer behavior.
The accepted artifact starts at the literal baseline SHA with a thin old-signature adapter and common benchmark protocol.
Keep its production tree unchanged. The candidate calls the current full-call convenience API.
Five forward-conversion controls preallocate coordinates and exclude preparation, allocation, and inverse rendering from timing.

The website quantizer returns enabled-color objects and string warnings, and its matcher uses JavaScript-number coordinates.
A faithful five-space indexed adapter is not established. Reject TypeScript quantize claims explicitly; label public comparisons package controls.

## Authorized measurement budget, execution pending quiet-phase clearance

Use a deterministic 128×96 fixture, two AB/BA pairs, 20 samples, 50 ms warmup, 250 ms cap, and 2 ms throughput.
Native cases include five forward conversions, five quantize spaces with palette 64, sRGB palettes 16/256, and sRGB throughput 64.
Public package controls include five spaces with palette 64 and sRGB throughput 64, giving six cases per engine.
The 13 native cases and six public cases across three engines cap execution at 124 sequential workers.
Declare artifacts and settings before the coordinator's quiet phase. This task runs no measurements.

## Protocol validation

`cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins` passes.
Focused Rust tests pass with 2 quantize identity/scope, 11 browser protocol, and 5 worker fixtures.
`node --test scripts/benchmark-public-browser.test.mjs` passes 10 controlled fixtures, including indexed metadata preflight.
No real browser or benchmark executes in these checks. Native typed operations remain fail-closed until their workload adapter lands.
