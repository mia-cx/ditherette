# Held scalar-default rollout

This is S44 preparation for issue #86, not rollout approval. Keep this PR unmerged and undeployed until Mia accepts stability. The [S43 release blockers](../../../.plans/85-release-readiness.md#required-release-blockers) remain open.

## Intended configuration

The website initializes the public processor with scalar defaults. Normal processing runs one backend. Only development builds honor `VITE_DITHERETTE_WASM_PROCESS=false` as the temporary legacy override. Unset or `true` selects Wasm in development. Production ignores this variable and exposes no backend selector or comparison control.

The existing worker still owns scheduling, cancellation, source identity, progress, and stale-result rejection. The package API, scalar/threaded artifacts, kernels, frozen references, and permanent guard do not change.

## Temporary fallback

Package loading and typed initialization/capability failures activate the existing page-session TypeScript fallback. The existing progress message reports activation. Replacement workers inherit that session choice.

Each fallback request must pass `faithfulTypeScriptFallback`. Its current admission rules require nearest resize, preserve alpha, sRGB, and no dither. They also check exact resize coordinates and the visible pixel/palette relationship. Other requests fail visibly rather than substitute an unproven algorithm.

Processing failures, invalid input, memory limits, and callback errors do not activate fallback. This includes an initialization-shaped error thrown during processing. Cancellation still terminates active workers and retains the last valid preview; pending initialization cannot process a canceled request.

The developer override is distinct from automatic fallback. It permits legacy diagnostics without claiming package equivalence.

## Activation and rollback

Mia's stability acceptance is required before enabling this change. Passing these focused tests does not clear the 18 confirmed performance regression cells or 21 missing scalar release cells. Capped-output transport failure, unapproved bilinear drift, noisy comparisons, and threaded WebKit cleanup also remain blockers. S43 records the remaining size, publisher, visual, and retirement holds.

Before an authorized rollout, record the currently deployed website's immutable build/deployment identifier and retain that complete artifact. That deployment, not this branch's Git parent by assumption, is the rollback target.

If an accepted rollout needs rollback, restore that previous website build through the normal deployment process. Restore its worker, JavaScript, and Wasm assets together. Reload open pages so their workers and page-session fallback state belong to the restored build. A production environment variable cannot select TypeScript, and fallback is not a substitute for deployment rollback.

No deployment, merge, package publication, release tag, or activation occurs during S44 preparation. S45 retirement remains a separate held descendant PR.

## Validation and evidence reuse

Parent is S43 PR132 at `15300c0dc461265fcef2bd72096de5202836706b`. The gate and focused tests are commit `dd78f749eaf3b9519f1743ba662c4ee293c26794`. The production override test first fails with the old TypeScript output, then passes after the one-line change.

Fresh checks pass without skips:

- 70 server checks cover production/development selection, faithful fallback refusal, visible errors, cancellation, stale results, schemas, and stores.
- Six Chromium checks use the actual package for website processing and admitted fallback equivalence. They cover production ignoring the developer override, decoding, crop packing, modes, persisted indices, rendering, and export.
- Focused Prettier and ESLint checks pass.

The browser checks use S43's retained ordinary tarball, SHA-256 `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90`. Its build source remains `c43cea1269fcd666835d41c07d82a1c451604107`, not S44. Extracted distribution files match the S42/S43 inventory byte for byte. The package, Rust, frozen guard, and locked dependency inputs have no diff against the validated parent.

Reuse the [S43 package/conformance/memory/size evidence](../../../.plans/85-release-readiness.md) only for those unchanged inputs. S41 measurements retain their original revisions and unresolved gates. No Rust build or benchmark is repeated, and this report claims no fresh rollout timing.

From the isolated worktree, after frozen offline installation and extracting the verified tarball's `package/dist` into `packages/ditherette`:

```sh
pnpm exec svelte-kit sync
pnpm exec vitest run --project server src/lib/processing/worker-pipeline.spec.ts src/lib/processing/client.spec.ts src/lib/processing/package-fallback.spec.ts src/lib/processing/schemas.spec.ts src/lib/stores/app.spec.ts
pnpm exec vitest run --project client src/lib/processing/package-pipeline.browser.spec.ts src/lib/processing/package-fallback.browser.spec.ts
pnpm exec prettier --check src/lib/processing/worker-pipeline.ts src/lib/processing/worker-pipeline.spec.ts src/lib/processing/package-pipeline.browser.spec.ts src/lib/processing/package-fallback.browser.spec.ts
pnpm exec eslint src/lib/processing/worker-pipeline.ts src/lib/processing/worker-pipeline.spec.ts src/lib/processing/package-pipeline.browser.spec.ts src/lib/processing/package-fallback.browser.spec.ts
```

The committed reports retain compact evidence. Finished generated website, browser-cache, and package outputs are removed at handoff. S43's retained tarball remains available outside target directories.
