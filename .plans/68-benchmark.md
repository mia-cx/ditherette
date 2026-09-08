# S27 blue-noise benchmark baseline

Completed trial results and delivery status are in [68-benchmark-results.md](68-benchmark-results.md).
The preparation checkpoints below retain their original holds and artifact identities as history.

Issue #68. Validated blue-noise parent `e4a44b718d49c48902806c0a35726ca1acac06ef`
joins accepted S26 delivery `eeb0ba13ec452a26780400b6b7f569c3f7730578` at `28c745a0`.
Documentation-only join `d99ee390` also includes delivered S26 PR #115 head `bb36452ca831bad485f924e7ee007f9bbdb1cb0d`.
Mechanical conflict resolution preserves every blue-noise runtime, frozen spec, shared image, Wasm interface, and package byte.
S26 contributes its benchmark adapters, repaired field verifier, and completed baseline-retention evidence.

## TODOs

- [x] Register the actual fixed blue-noise lookup and complete Processor/public adapters; verify exact frozen outputs through the full verifier.
- [x] Declare 68 serial workers and export matching installed-package fixtures; pass untimed native checks.
- [x] Commit clean code and prepare independent fresh native/public role builds with all jobs drained.
- [x] Use an independently identified frozen-only Wasm oracle for browser preflight; pass exact public conformance while retaining native diagnostics.
- [x] Prepare fresh independent S27 roles and immutable pairs after rebasing onto the actual updated S26 base; hand off unstarted workers.
- [x] Audit the coordinator's completed 68-worker trial and record exact verification, actual samples, medians, and limitations.

## Fixed scope

The accepted implementation already uses an O(1) 32×32 lookup. No new optimization candidate is proposed.
Both roles independently rebuild the same clean revision to establish the exact baseline and check integration stability.
The coordinator owns measurements; preparation never launches a measurement worker.

Use a varied 65×33 RGBA image to cross two tile column boundaries and one tile row boundary.
Register one native threshold-grid component and four complete recipes:

1. sRGB perturb, everywhere.
2. Oklab perturb, adaptive radius 2, threshold 10, softness 5.
3. YCbCr perturb, everywhere.
4. YCbCr perturb, adaptive radius 1, into Oklch hue-arc matching with S24's ordered palette64.

Every recipe uses the fixed blue-noise field and strength 0.7.
Separable matching preserves alpha with threshold 0.5.
Five native cases plus four cases in each of Chromium, Firefox, and WebKit total 17 cases.
Two alternating accepted/candidate pairs give 68 serial workers.
Each case uses 20 single-call samples and 50 ms warmup, with no application cache.
The threshold cap is 250 ms; every complete call has a predeclared 10-second cap.
The shared full verifier requires exact threshold bits, RGBA bytes, indexed bytes, and metadata.
Public conformance also compares actual `quantize(perturb(...))` outside timing.

## Artifact and ownership limits

Own benchmark adapters, declarations, fixtures, tests, and this plan only.
Keep production, frozen spec, image storage, and public package code unchanged after the mechanical join.
The native cache is `.worktrees/v1-s24-bench/crates/ditherette-bench/target`.
Preserve all retained `target/s24-*` evidence.
For both public role builds, link only local Wasm target `scalar` and `threads` children to
`.worktrees/v1-s23-trilinear/crates/ditherette-wasm/target/{scalar,threads}`.
Return those exclusive caches to the coordinator after preparation and validation finish.
Fresh preparation uses existing source-inventory and build-provenance checks.
The original preparation task excludes measurements and external writes.
The final delivery task records the coordinator's completed trial and files the unmerged S27 PR.
PR merging, publishing, deployment, and candidate selection remain outside its scope.

## Code validation checkpoint

The existing field registry now dispatches BlueNoise to actual production/reference lookup functions.
Complete Processor and public recipes accept the already-delivered blue-noise mode through their existing typed adapters.
No extra runtime implementation or table copy was introduced.
The full verifier passes exact 65×33 threshold and complete outputs, including crossings at 31/32 and 63/64 columns and 31/32 rows.
Native and public identities match, and perturb alpha remains unchanged.

Twelve focused Rust tests pass across blue-noise adapters, field adapters, browser workers, and the fixed matrix example.
The retained S26 replay test remains opt-in and was not rerun.
Twenty-six Node browser/timing protocol tests pass. Rust bins/examples checks and both formatting checks pass.
Declarations and four frozen installed-package fixtures were generated in `target/s27-preparation/` without timing.
Set `DITHERETTE_BENCH_BLUE_NOISE_FIXTURES` to its `blue-noise.json` for the existing public conformance suite.
That suite checks all four new recipes with primed/fresh instances, including actual separable composition and durability.

## Prepared builds and exact-conformance hold

Clean code checkpoint `e395b16241f26443a43a049a0fbefb7166215fb8` independently rebuilt both roles.
Retained paths are relative to `.worktrees/v1-s27-bench/target/s27-preparation/`:

- `accepted-native/` and `candidate-native/` contain fresh worker/coordinator binaries and build provenance.
- `accepted-public/` and `candidate-public/` contain fresh scalar/threaded packages, installed consumers, bundle sources, and build provenance.
- Both `ditherette.tgz` files have SHA-256 `32a844782798e87c8e561fd2ca9dc2fd5d653db94ff65648c14702b92f5ecfe1`.
- `native.json`, `public.json`, and `blue-noise.json` retain the fixed declarations and four native-frozen fixtures.

Untimed public checks expose one native/Wasm difference in the Oklab adaptive2 recipe.
At pixel `(8,26)`, RGBA offset 6792, native frozen and actual release-native Processor produce red 95; both public roles produce 94.
The other 8579 bytes and all three other complete recipes match exactly.
Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 show the same result.
The existing smaller S26/quantize checks passed before this larger S27 fixture failed.
This failure does not invalidate that smaller scope, nor does it establish exact conformance for this larger scope.

`{accepted,candidate}-{chromium,firefox,webkit}-diagnostic.json` preserve every full actual/reference output and nine forward-coordinate bit patterns.
`native-diagnostic.json` preserves all four release-native actual outputs and the focal stage comparison.
The diagnostic scripts remain beside these ignored artifacts.
Only the focal pixel's forward Oklab coordinates differ; its eight adaptive neighbors match bitwise.
Input RGBA is `[73,85,65,70]`.
Native coordinate bits are `[1054713084,3167088128,1020538528]`; Wasm bits are `[1054713084,3167088160,1020538512]`.
The adaptive mask remains bit-identical at `1056450327`.
Substituting only Wasm forward coordinates into unchanged native placement/reconstruction moves unrounded red
from `94.50003118629185` to `94.49999517762998`, which explains the final-byte difference.
This localizes the observed mismatch to shared forward color math, without changing the blue-noise lookup.

Keep the native-frozen fixtures and all mismatches intact. No tolerance, fixture replacement, or production fix has been applied.
The coordinator independently confirmed that the unchanged frozen Wasm oracle matches the complete actual 8580-byte frame.
Immutable pair snapshots and all 68 measurement workers remain unstarted.

## Same-target reference protocol

`crates/ditherette-bench-oracle` is a standalone, unpublished test crate.
Its semantic modules reference only the frozen spec and image trees, with no core/production crate dependency.
Its typed adapter covers resize, quantize, perturb, separable, and the S28/S29 diffusion/Yliluoma wire settings.
Algorithms remain in the unchanged frozen modules.
The adapter independently validates the source and settings and computes the existing complete `CaseIdentity`.
Native benchmark references and identity serialization remain unchanged.

Fresh public preparation builds the oracle into the existing `scripts/oracle/` asset closure, outside the npm tarball.
The existing build provenance binds every tracked input and emitted script/Wasm byte.
An oracle manifest uses the same `BuildFile` records to bind its exact inputs to that complete provenance.
Preparation checks frozen content, resolved dependency versions/features/checksums, compiler/binding-tool hashes, standard-library hashes, and emitted files.
The trusted manifest checker requires the exact release profile and rejects profile overrides, patches, workspaces, and build scripts.
Oracle-local Cargo configuration and toolchain redirects fail before metadata.
Both local oracle/API crates rebuild; registry caches remain intact.
The explicit `frozen-build` feature reproduces the approved core-Wasm Serde JSON features without changing the separately frozen native benchmark feature closure.

The browser evaluates the oracle in a separate disposable context.
That context closes before actual package initialization, preflight, warmup, or sample timers.
The transport retains the oracle's full case identity and exact output alongside actual output.
Rust rejects identity drift and uses the verified same-target reference for exact bytes, indices, palette, transparency, and warning metadata.
Mismatch/instability bundles use that same reference. The response bound reserves three full outputs for reference plus both unstable actual outputs.
Each new transport also writes `reference-diagnostics.json` with native and Wasm references and the bound artifact identity.
No expected-output override or numeric tolerance was added.

## Protocol validation and handoff

Protocol code checkpoint is `aee348cb4e088127a3780bbca66760ed3370cf25`.
The missing-Wasm-reference regression failed on the old protocol before implementation.
Focused checks cover native identity/output parity, all current resize identities, diffusion/Yliluoma typed mapping,
source/settings/semantic identity mutation, one-byte mismatch preservation, three-output size bounds,
frozen/source/binary substitution, local build redirects, profile overrides, and scoped fresh-build cleanup.

Thirty-two focused Rust tests and 29 Node protocol/preparation tests pass, including the enabled profile-mutation test.
Rust bins/examples checks and formatting pass. Frozen content/compiler/dependency/syntax checks pass with unchanged trusted policy.
Three engines pass 47 quantize and 16 field fixtures through primed/fresh actual installed-package adapters,
including 12 actual `quantize(perturb(...))` compositions per engine.
The suite asserts every oracle context is closed before creating the package context.

Untimed validation artifacts remain under `target/s27-preparation/`:

- `oracle-validation-03/` contains the frozen-only JS/Wasm and manifest; Wasm SHA-256 is `e3049543fbfefee45608e4893ebd98e2e3d0647f1187e41683d0aad2b91279df`.
- `oracle-browser-evidence-03/{chromium,firefox,webkit}-references.json` retains full native/target references, engine versions, oracle manifest, and installed tarball digest.
- `*-identified.json` fixtures add full native identities without replacing the original fixture files or outputs.

These checks use the unchanged retained S27 installed package. They are untimed validation, not fresh measurement provenance.
The earlier `e395b162` role artifacts predate this protocol and must be rebuilt before new trials.
No immutable pair snapshot or measurement starts here; the coordinator owns fresh role preparation and the eventual quiet phase.
When joining S28/S29, preserve their indexed-output classification for both Diffusion and Yliluoma.
Their full fixture suites and fresh joined-role provenance remain integration checks, beyond the typed mapping tests here.
All build and browser jobs are drained; S23 scalar/threaded caches have returned to the coordinator.
The coordinator also holds new snapshots for a separate output-stability evidence fix from PR #113.

## Rebased S27 paired preparation

The immutable comparison-evidence fix `eb1725ef8bf8eadaf703d3e0a93576c2bf9a70eb` joined as `a9327c68560b654b912977102e9fb8036f5f9eeb`.
Twenty-seven browser/timing protocol tests pass after that join.
The branch then rebased with `--rebase-merges` onto actual updated S26 base `68f058e93dce025ba87fa728f0b960a09dc96948`.
Rebased head `d27df659335774456532a207772b5c6afb8520cf` retains the complete pre-rebase tracked tree byte-for-byte, including merge-only documentation.
The duplicate snapshot-fix cherry-pick dropped because its patch is now in S26 ancestry.

Prepare `target/s27-trial-01/` with two independent native builds and two independent public builds from the same clean final source revision.
The public builds include the separate frozen-only oracle and immutable output-stability protocol.
Validate both installed roles with the original four S27 recipes and retain full native/Wasm reference diagnostics.
Use the unchanged 68-worker declaration and existing immutable native/browser preparation commands.
Preparation may snapshot all three engines but must not launch measurement workers.
The S24 native cache and S23 scalar/threaded caches are exclusive to this preparation until the final drain handoff.
Preserve every earlier preparation, diagnostic, and trial artifact.

### Shared-backing guard and final preparation

The collector now rejects shared output backing before retaining immutable comparison snapshots.
Fix `f8a2cc11dc42e4815ea8cffbb1b3f36c1a116395` joined through updated S26 parent `59036e1aef87943e462b4cce6b371e5edd082979`.
The branch rebased with merges preserved. Rebased `625ff0867f77d4d0f65a5301c89833d2d96c2a87` exactly matches the prior joined tracked tree.
Twenty-eight focused browser/timing tests pass with the shared RGBA, indices, and palette regressions.

Retain `target/s27-trial-01/` as superseded preparation evidence from source `518a4f6d6c40f76928a13dc38f71866605e671bc`.
Its four independent role builds and both roles' three-engine exact conformance passed.
Each engine retains 63 native/Wasm references, including the unchanged Oklab byte 6792 difference of native 95 versus Wasm 94.
Only its native snapshot was prepared. No browser snapshot or measurement worker started before the shared-backing hold.

Prepare final roles and immutable snapshots under new `target/s27-trial-02/` from the clean revision containing this declaration.
Keep that source fixed through both independent role builds, provenance checks, untimed conformance, and all four snapshots.
Use the same 68-worker matrix. Leave all measurement workers unstarted for the coordinator's quiet-phase clearance.
