# Website source consolidation

This local branch is based on `91b114ba` and records the optional website source stack without release or verification ancestry.

## Applied ranges

- Issue [#80](https://github.com/mia-cx/ditherette/issues/80), PR #120: net tree `f408bc99..0305456b`.
- Issue [#81](https://github.com/mia-cx/ditherette/issues/81), PR #125: net tree `2a023768..5938b248`.

The package adapter, worker route, stale-request cancellation, page-session initialization fallback, and their focused tests are retained. PR #130 release files and PR #129 verification files are not included.

The independent `91b114ba` source branch deferred its website-only conformance step because it did not carry the verification workflow. After composition, that step is included in the package-conformance workflow. No rollout/default flip or TypeScript retirement is included.
