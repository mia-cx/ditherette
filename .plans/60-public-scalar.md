# S19 public scalar wrapper

Issue #60. Wrapper base `f36a67e6ff9f4246b83be8525a68d61057ee4c66` joins the delivered factory and literal-copy baseline/policy head.
This task owns package TypeScript, interface/tarball fixtures, and package build scripts.
The other S19 owners provide the private Rust processor and nearest optimization evidence.

## TODOs

- [x] Add the nearest request/result types, structured errors, and canonical raw-JS validation with focused fixtures.
- [x] Connect isolated lazy scalar initialization and guarded synchronous resize/disposal to the agreed private ABI.
- [x] Verify real public calls, failure boundaries, durable outputs, and installed-tarball browser loading.
- [ ] Record runtime provenance, rerun the focused checks, and deliver the final clean checkpoint.

Public requests preserve version one, `source`, and `output.resize.algorithm` from the frozen contract.
Only nearest resize and disposal are implemented here. `preferred` threads uses scalar; `required` reports capability until S34.
Supplying `onProgress` reports unsupported-operation until S33. It is never silently ignored.
The active-call guard starts before reading request properties. Expected failures preserve the instance.

Use the package-owned browser fixture, not the website preview. Node tests are development fixtures, not public Node support.
No benchmarks, PR creation, merges of GitHub PRs, publishing, tags, or deployments.

The five validation fixtures pass through `pnpm --filter ditherette test:validation`.
They cover all nine anchors, offset/detached bytes, canonical object/string tags, extra fields, dimensional limits,
explicit unsupported progress/filter settings, initialization defaults and limits, and single-read property handling.
The production ABI uses module-local functions rather than wasm-bindgen classes; each private factory owns that module state.

The final sink ABI joins from `a1ca26f09aa41dc13c29940bb19731b7845279be`, including native processor `342c0c46`.
The helper writes a complete image to a private JS sink and returns void. This avoids the reproduced caught-owned-return externref leak.
The wrapper reads that sink only after status zero. Stable numeric code/path mappings allocate no Rust error strings.

After that exact join, `pnpm --filter ditherette build` passes for both variants and package staging/compilation.
All 12 validation/public fixtures and the public type fixture pass through `pnpm --filter ditherette test:interface`.
Coverage includes exact capacity and one-under rejection, offset/detached views, reusable Response bodies, concurrent initialization,
durable results, getter/copy-boundary reentry, disposal, repeated caught-copy failure recovery, and bounded externref capacity.
The built root imports successfully with WebAssembly absent and fetch/Worker replaced by throwing functions.
Creating afterward returns capability at `wasm`. Public runtime exports are exactly `createDitherette` and `DitheretteError`.

Installed-tarball tests pass in Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4 through Playwright 1.59.1.
Each browser checks all nine known-vector anchors, eight custom initialization inputs, isolated failures, durable output,
scalar operation without isolation headers, inert root imports, and absence of threaded network requests.

The packed-artifact fixture found wasm-pack's generated `.gitignore` excluded all staged Wasm files from the tarball.
Package staging now omits that ignore file; original crate and website artifacts stay unchanged.
Chromium also rejects DataView directly in WebAssembly.instantiate. Custom views now become offset-preserving Uint8Array views without byte copies.
WebKit uses private extracted Debian libraries and a task-local launcher because its bundled launcher replaces LD_LIBRARY_PATH.
No system packages or shared browser binaries changed. Exact provenance follows in the runtime evidence file.
