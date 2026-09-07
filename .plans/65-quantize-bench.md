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
- [x] Register prepared quantize and five forward adapters, reusing the native timing loop and exact three-way verifier.
- [x] Declare the bounded matrix and validate the native checkpoint without measurements.
- [x] Create the literal-baseline worktree with common benchmark protocol and thin old-signature adapter; leave production bytes unchanged.
- [x] Join validated public quantize and verify the actual installed-package benchmark adapter without timing.
- [x] Prepare immutable accepted/candidate artifacts as attempt 01 and hand off without measurements.
- [~] Integrate public retained-batch verification, then rebuild affected artifacts before coordinator-authorized measurements.

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

## Native checkpoint

The existing resize warmup, calibration, measurement, and observer loop now accepts a callable workload.
Resize keeps its caller-owned RGBA buffer and existing result schema. Quantize creates and drops each actual `IndexedImage` inside timing.
The full native quantize wrapper borrows source bytes and uses the current convenience API with an unrestricted preparation budget.
The accepted artifact will call its old one-argument convenience API under the same timing scope.
Contract mapping, frozen execution, exact output serialization, and final stability checks remain untimed.
Unstable native output keeps the request, both actual outputs, reference, and raw samples beside the request as `.unstable.json`.

Five forward controls reuse landed `Converter::rgba8_into` with prepared tables and caller-owned f32 triplets.
Actual packed coordinates and byte alpha use the existing numeric verifier.
The frozen per-space inverse only renders untimed diagnostics because production exposes no inverse API.
No production-as-reference adapter or archived production implementation was added.

Validation commands pass:

```sh
cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --tests --example quantize_integration_plan
cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --target wasm32-unknown-unknown
cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml -- --check
cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check
node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s24-bench --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze
```

The Rust suite passes 56 tests, with one subprocess-only fixture ignored in the parent test run.
Its lease checks separately invoke that fixture twice and run three controlled Node cleanup tests.
The new adapter fixtures compare 46 quantize combinations and five packed color controls exactly with frozen outputs.
The registry reference-count fixture now counts only `spec` conformance subjects, allowing real candidate registration.
The guard retains frozen revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and digest `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Prod, spec, image, and freeze-policy files remain unchanged by benchmark work.

The coordinator ended S23's quiet phase before this task resumes. No S24 measurements are authorized.
Validated public package `0ae8b95f5355b5f311b474034faf2f2bfb686eb5` joins cleanly at `5bba0c25`.
The separate baseline worktree starts at the exact literal checkpoint and shares only the benchmark protocol.
Its ten focused tests and trusted guard pass; original production, spec, image, and policy bytes remain identical.

`quantize_conformance` exports 17 small frozen indexed fixtures for the real browser adapter.
The installed-package test checks all five matching spaces, all alpha policies, palette warnings, fresh/primed instances, and durable results.
The fixture generator compiles and the ten controlled browser protocol tests pass before package artifact preparation.
Actual installed-package conformance passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each engine checks all 17 frozen fixtures with both primed and fresh instances, without calling the timing loop.
The final 56-test Rust suite, Wasm check, formatting, and trusted guard pass after the public join.

[Attempt 01](65-prepared-attempt-01.md) records immutable artifact paths and complete hashes.
The coordinator pauses measurements after review finds that endpoint-only checks miss intermediate A/B/A output changes.
Public retained-batch verification is being fixed separately. Native full-call timing drops each output inside the timed call.
The coordinator accepts the fixed deterministic native callable scope with before/after conformance only, not every timed output.
This cannot certify arbitrary stateful/nondeterministic callables or transient A/B/A behavior.
Native disposal and timer boundaries remain unchanged; broader observation requires a separately declared scope or mechanism.
Attempt 01 remains unmeasured historical preparation, not validated performance evidence.

## Attempt 02 plan

Join the clean stability correction `debaa849f9041657ccc9b6942f561213d25d5dbc` onto coordinator handoff `ee1dc6e8874ef2a9fb26a3b61934a9f6d0b10c32`.
Extend its observer to indexed buffers and warning metadata, preserving the shared collector and its 64 MiB retained-result cap.
Compare every returned indexed result outside the timer; retain the first and first distinct outputs and reject reused writable storage.
Keep calibration, native destruction costs, and native checked-endpoint policy unchanged.
Run controlled protocol tests and untimed installed-package conformance before creating new attempt 02 snapshots.
Reuse the immutable native accepted worker only after confirming its unchanged wire protocol remains compatible.
Preserve attempt 01 and the fixed 124-worker budget. The coordinator alone may authorize measurement.

The indexed observer shares `equalOutput` with preflight and serializes only final evidence.
Each retained record reserves pixel-count index bytes, 1,024 palette bytes, and the collector's 1,024-byte metadata allowance.
It accepts at most three known warnings, each with at most 88 characters, matching the frozen warning vocabulary.
It rejects extra indexed metadata, oversized backing buffers, and reused result, palette, warning-array, or warning records.
Both result formats reject reused result objects even when each call replaces its buffers.
The public subject contract requires independent records and durable owned buffers. This is not a total JavaScript heap limit.
Every warmup and measured output reaches the observer after its call or complete batch timer.
Retention changes allocation lifetime and GC behavior. New artifacts must not be pooled with attempt 01 samples.
Native protocol and timer source compare byte-identical with accepted `ccb9bceb28563c562dd5e6c05f68c056c18e3519`.
The accepted immutable executable can therefore be reused without rebuilding its historical source.

The merged checkpoint passes 56 Rust tests, 26 controlled JavaScript tests, Wasm compilation, formatting, and the trusted guard.
One Rust fixture is ignored in its parent suite and runs twice as a controlled child; three Node cleanup fixtures also pass.
Indexed A/B/A fixtures cover indices, palette bytes, transparency, warning code/text, alias rejection, and metadata-only worker evidence.
The installed-package conformance suite now invokes this same observer on each untimed indexed output.
