# S26 and restored resize integration

Join `82a7e3e9` combines coordinator `8af3a9c9` with S26 PR115 head `bb36452ca831bad485f924e7ee007f9bbdb1cb0d`.
S26 retains its original exact field implementation. The converter candidate remains outside this ancestry.

## Checks

- [x] Preserve the coordinator's complete production resize tree byte-for-byte.
- [x] Preserve S26 production color, palette, quantize, and dither trees byte-for-byte.
- [x] Resolve browser subject conflicts by retaining both trilinear and perturb/separable registrations; combine package capability prose.
- [x] Pass all 327 native tests with `bench-subjects`, 22 focused benchmark tests, and 28 controlled Node protocol tests.
- [x] Freshly rebuild scalar/threaded package variants using the coordinator's own caches.
- [x] Pass 26 public interface/type tests and 12 private ABI tests.
- [x] Pass installed-package tests and actual untimed benchmark adapter checks in Chromium, Firefox, and WebKit.
- [x] Pass both Rust format checks, whitespace checks, and the trusted frozen content/isolation guard.

The first focused benchmark invocation named two nonexistent test targets and ran no benchmark tests.
The corrected command uses the existing field, paired-browser, verification-adapter, and quantize-adapter suites. All 22 pass.

The installed package retains 91 frozen field vectors and 1,365 exact compositions alongside all resize and matching modes.
The actual adapter checks 47 quantize fixtures, 12 field fixtures, and 10 compositions per browser, plus trilinear integration.
Browsers are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Tarball `target/s26-join-validation/ditherette.tgz` has SHA-256 `c3a0b194cbff4f9365cedc1ba88f0ce24321e2197aaf39558d64fa334bb0e812`.

Frozen revision remains `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
These are combined conformance checks, not fresh performance evidence.
Completed root-owned debug package outputs were cleaned to recover disk space. Immutable benchmark artifacts and release outputs remain intact.
