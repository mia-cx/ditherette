# #87 Held TypeScript retirement

Parent S44 PR133 is `impl/v1-s44-rollout` at `76bf1f8938c14a1a4cde290d45c61a280f7434d2`. This is a separate held retirement diff. Actual rollout acceptance and unresolved release gates remain prerequisites to activation.

## TODOs

- [x] Retire live legacy execution and fallback selection, retaining website helpers and retryable package-only processing with focused checks.
- [x] Replace failed workers so retries also clear cached module-import and Wasm-compilation failures.
- [ ] Record exact package evidence reuse, preserved browser responsibilities, and held rollback/activation boundaries.
- [ ] Root joins the separately owned benchmark provider, validates the complete diff, and files the held unmerged PR.

## Ownership and findings

This worktree owns website source and focused tests. The provider agent owns benchmark preparation/provenance files in its separate worktree. Root owns their join, global tracking, and PR filing.

The current page-session fallback latch prevents initialization retry. The worker also caches rejected initialization promises. Remove the latch and reset only the failed initializer; subsequent existing processing requests retry. Preserve processing errors and the last valid preview. No new UI or retry control is introduced.

Move the adapter's small crop/matte/strength helpers into existing adapter/types ownership before deleting legacy modules. Preserve Bayer/color helpers used by previews and timing-history helpers. Check live imports before removal. Keep standalone raw Wasm benchmark/staging tools that do not depend on the retired loader.

Reuse the verified ordinary S43 tarball. No Rust build or performance measurement is authorized or needed for unchanged package/runtime inputs. Preserve frozen spec, image, guard, and production kernels.

The retry test first fails because initialization resolves to the old fallback protocol. The package-only worker now clears failed initialization promises and succeeds on a later request. Processing failures remain visible and do not reset a successful initializer.

The website baseline passes 123 server checks and four actual Chromium package checks. Focused ESLint and diff checks pass. Svelte checking reports zero errors and one warning for absent ignored `worker-configuration.d.ts`. Obsolete backend tests are removed; the two retained source/output-bound tests move into `types.spec.ts`. The live source import scan has no retired backend, selector, or fallback references.

Root authorizes joining provider commit `a58f5bee4c8a336f1ba11cd199be01b050c47231` after this clean baseline, then combined validation. PR filing remains root-owned.

The website baseline is `ad6ff442a235ffbdca5940bc3f8ec3090efa99bd`; merge `2b73107dd5e9386ab9c14a337bfa0df590b9a2e6` joins the provider without conflicts. A focused retry test then proves the cached-module boundary needs worker replacement. The existing client error handler now terminates the failed worker, preserves its visible error and last preview, and reloads the source on the next request. No new protocol field or retry control is needed.
