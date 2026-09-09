# #87 Held TypeScript retirement

Parent S44 PR133 is `impl/v1-s44-rollout` at `76bf1f8938c14a1a4cde290d45c61a280f7434d2`. This is a separate held retirement diff. Actual rollout acceptance and unresolved release gates remain prerequisites to activation.

## TODOs

- [x] Retire live legacy execution and fallback selection, retaining website helpers and retryable package-only processing with focused checks.
- [x] Replace failed workers so retries also clear cached module-import and Wasm-compilation failures.
- [x] Record exact package evidence reuse, preserved browser responsibilities, and held rollback/activation boundaries.
- [x] Join the separately owned benchmark provider and validate the combined implementation.
- [ ] Root records final ancestry and files the held unmerged PR.

## Ownership and findings

This worktree owns website source and focused tests. The provider agent owns benchmark preparation/provenance files in its separate worktree. Root owns their join, global tracking, and PR filing.

The current page-session fallback latch prevents initialization retry. The worker also caches rejected initialization promises. Remove the latch and reset only the failed initializer; subsequent existing processing requests retry. Preserve processing errors and the last valid preview. No new UI or retry control is introduced.

Move the adapter's small crop/matte/strength helpers into existing adapter/types ownership before deleting legacy modules. Preserve Bayer/color helpers used by previews and timing-history helpers. Check live imports before removal. Keep standalone raw Wasm benchmark/staging tools that do not depend on the retired loader.

Reuse the verified ordinary S43 tarball. No Rust build or performance measurement is authorized or needed for unchanged package/runtime inputs. Preserve frozen spec, image, guard, and production kernels.

The retry test first fails because initialization resolves to the old fallback protocol. The package-only worker now clears failed initialization promises and succeeds on a later request. Processing failures remain visible and do not reset a successful initializer.

The website baseline passes 123 server checks and four actual Chromium package checks. Focused ESLint and diff checks pass. Svelte checking reports zero errors and one warning for absent ignored `worker-configuration.d.ts`. Obsolete backend tests are removed; the two retained source/output-bound tests move into `types.spec.ts`. The live source import scan has no retired backend, selector, or fallback references.

Root authorizes joining provider commit `a58f5bee4c8a336f1ba11cd199be01b050c47231` after this clean baseline, then combined validation. PR filing remains root-owned.

The website baseline is `ad6ff442a235ffbdca5940bc3f8ec3090efa99bd`; merge `2b73107dd5e9386ab9c14a337bfa0df590b9a2e6` joins the provider without conflicts. A focused retry test then proves the cached-module boundary needs worker replacement. The existing client error handler now terminates the failed worker, preserves its visible error and last preview, and reloads the source on the next request. No new protocol field or retry control is needed.

## Integrated evidence

Validated source is `80c62ef564503b6703a3fe47373c32b1dcb38449`. [Machine evidence](87-retirement-evidence.json) records exact hashes and inventories. Later documentation commits do not replace this validation identity.

`node scripts/prepare-benchmark-typescript.mjs target/s45-typescript` succeeds from that clean joined source with the live legacy files absent. It compiles 24 historical inputs. All 24 emitted JavaScript files match S41's recorded bytes. The provider revision remains `a895267baea624a6e89bfcef6c5147f170e8a8f7`, distinct from the current checkout. The historical TypeScript provider is not the frozen Rust correctness oracle.

The ordinary package remains S43's artifact, built from `c43cea1269fcd666835d41c07d82a1c451604107`. Its SHA-256 is `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90`. All 45 extracted distribution files match the S42/S43 inventory. Package sources, Rust, frozen spec/image/guard, toolchains, and locked dependencies have no changes. Reuse their [S43 conformance, memory, and size evidence](85-release-readiness.md), not an invented S45 rebuild or measurement.

Combined validation passes without skips:

```sh
node --test scripts/prepare-benchmark-typescript.test.mjs scripts/prepare-public-benchmark.test.mjs
pnpm exec vitest run --project server src/lib/processing src/lib/stores/app.spec.ts
pnpm exec vitest run --project client src/lib/processing/package-pipeline.browser.spec.ts
pnpm exec svelte-check --tsconfig ./tsconfig.json
```

Results are eight preparation checks, 123 server checks, and four Chromium checks. Svelte checking has zero errors and one missing generated Cloudflare-types warning. Focused ESLint/Prettier and `git diff --check` also pass. The existing S43 tarball supplies package assets after frozen offline dependency installation and SvelteKit sync. No Rust build or performance benchmark runs.

## Preserved responsibilities and holds

Decode, crop packing, palette metadata/selection, preview color/Bayer helpers, rendering, PNG export, source persistence, identity hashes, stores, and UI remain. Source/output bounds retain their tests. Scheduling, cancellation, public progress counts, stale-response handling, durable results, and buffer transfer remain covered. Legacy cache/memory estimators are removed; timing-history helpers and their existing consumer types remain.

Removing the obsolete loader does not remove standalone raw Wasm staging, probing, or benchmark tools. The frozen Rust reference and conformance tooling remain permanent. Historical provider extraction reads Git objects, never substitutes the current website, and fails explicitly if history is missing.

Hold retirement until Mia accepts the actual initial rollout and no blocking regressions remain. The 18 confirmed regressions, 21 missing scalar release cells, capped-output transport failure, unapproved bilinear drift, noise, threaded WebKit cleanup, size review, publisher setup, and human acceptance holds remain open. Preparation does not satisfy any activation gate.

Rollback restores the complete previously accepted website deployment, including its worker and Wasm assets, then reloads open pages. Record that deployment identifier before any authorized rollout; do not assume the Git parent is the deployed rollback artifact. No backend switch or TypeScript fallback remains in this retirement diff. The existing error caption presents initialization failures; the next existing processing request retries through a fresh worker. No retry button or other UI is added.

## Handoff

All owned jobs have exited. Cleanup removes the complete target tree, generated website/package outputs, and Vite browser caches, about 42 MiB. No compiler target was created. Compact evidence remains committed; the original S43 tarball is untouched. Root owns final ancestry/tracking and PR filing. No merge to a published parent, deployment, publishing, release tag, or activation occurs.
