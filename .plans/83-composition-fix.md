# #83 Process composition initializer fix

Base: `585696c63647e0adba51c0ca644a3756bf8808d2`. Branch: `impl/v1-s41-composition-fix`.

- [x] Preserve the selected role and artifact during the untimed Process composition check, with focused mock validation.

The automatic Chromium case 7 failure reaches `runTrial`'s composition preparation. That code remaps the role while retaining the original Wasm asset. The selected thread policy can therefore disagree with the asset.

The existing fake-clock Process fixture exercises `runTrial` and `prepareOperation`. Cover both roles and both Process backends. Require the fixture's selected thread policy and record the fetched Wasm entry.

The failing test reached the same composition preparation line with `Wasm fixture requires required threads; got disabled.` The fix changes only the comparison backend and retains the original role. The four role/backend combinations pass afterward, including returned identity, sample count, disposal, and both fetched asset URLs.

Role-control audit: `threads` and `row_policy` select by trial role, so both now retain the original selection. Progress stays explicitly disabled for composition. Cache comparison stays disabled, application-cache stays not-applicable, and primed-sample still maps to fresh-instance for this untimed check. Measured-call preparation and output verification do not change.

Validation:

- Red: `node --test --test-name-pattern='Process composition keeps' scripts/benchmark-startup-trial.test.mjs` failed at the composition initializer before the fix.
- Green: `node --test scripts/benchmark-startup-trial.test.mjs scripts/benchmark-progress-trial.test.mjs scripts/benchmark-stage-cache.test.mjs scripts/benchmark-row-policy.test.mjs` passed 20 tests.
- Green: `node --test --test-name-pattern='explicit capped page|compact frozen mismatch|compact composition failure' scripts/benchmark-public-browser.test.mjs` passed 3 tiny mock tests.
- `git diff --check` passed. This worktree contains no `target/` directories or generated artifacts.

Kernel code, the public package, frozen content, artifact identities, and verification rules stay unchanged. No builds, browser runs, measurements, or PR creation belong to this task. The coordinator owns fresh artifacts and the bounded retry.
