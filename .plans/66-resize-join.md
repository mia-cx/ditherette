# S25 and resize integration

Join retained S25 baseline `bf1b887ca90409b909b742e5eb78f97ab00f5bbd` with coordinator `9958a394`.
The S25 parent already contains delivered S24 `884a8868`; the coordinator contains validated S23 trilinear.
The rejected dispatch candidate stays outside this ancestry.

## Checks

- [x] Preserve the coordinator's production resize tree and S25's production color, palette, and quantize trees byte-for-byte.
- [x] Resolve the browser assertion to retain 27 trilinear cases and 15 quantize cases.
- [x] Sort the merged benchmark module declarations with rustfmt's existing order.
- [x] Pass 317 native tests with `bench-subjects` and 33 controlled JavaScript tests.
- [x] Rebuild scalar and threaded package artifacts through `buildFreshPackage` before testing their new tags.
- [x] Pass 23 interface tests and type checks, 10 private ABI tests, and installed-tarball checks in all three engines.
- [x] Pass both crate formatting checks, whitespace checks, and the trusted frozen-spec guard.
- [ ] Validate the combined Rust benchmark adapters after their shared target returns from S25 delivery.
- [ ] Check the actual benchmark adapter against the joined tarball without collecting timings.

The first interface check used the previous S24 package artifacts and rejected the new matching tags.
Both new-tag tests pass after rebuilding the combined package. No implementation change addressed that stale build.

The trusted guard retains frozen revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`
and digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Browser versions are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
These are integration checks, not fresh performance measurements.
