# Verification source consolidation

This local branch is based on `91b114ba` and records a source-only extraction for the verification stack.

## Applied ranges

- Issue [#82](https://github.com/mia-cx/ditherette/issues/82), PR #129: net tree `5fccb9e6..95706738`.
- Issue [#83](https://github.com/mia-cx/ditherette/issues/83), PR #131: net tree `95706738..d09e32df`.
- Issue [#85](https://github.com/mia-cx/ditherette/issues/85), PR #132: net tree `c43cea12..15300c0d`, without importing the `c43cea12` merge ancestry.

PR #129's website cancellation and faithful-fallback workflow step is intentionally omitted here. It is retained for the optional website consolidation. PR #132's indexed-wire correction is included through `f331ffcf` in `packages/ditherette/tests/tarball-browser.test.mjs` and `packages/ditherette/tests/yiluoma-oracle-fixture.mjs`; its historical evidence documents remain historical.

Release PR #130 and website PRs #120 and #125 are not included. The known PR #129 memory high-water assertion gap is preserved for later review; this extraction does not repair it. No builds, benchmarks, or review actions were run.
