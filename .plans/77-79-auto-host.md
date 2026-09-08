# Ordinary automatic-policy host conformance

Parent checkpoint `a8418904fd9fd21ba757ab62254f0753e7420f21`.
Worktree `v1-row-auto-host`, branch `impl/v1-row-auto-host`.
Only host fixture scripts and this plan belong to this task.

## TODOs

- [x] Reuse existing asset staging, host exchange, restrictions, package loading, and complete-call adapters.
- [x] Add ordinary scalar/required-threaded comparisons for the nine selected S35/S36/S37 recipe classes.
- [x] Run Node syntax and controlled fixture checks, then commit for the coordinator's installed-artifact run.
- [ ] Run the coordinator-authorized ordinary-package Chromium/Firefox fixture once and report cleanup.

Both Wasm variants must come from an ordinary `build_mode: public` bundle and lack the developer override export.
The fixture uses real blocking-capable host workers and no private execution-policy calls or timing collector.
It compares exact bytes and metadata, checks public progress stages and pool disposal, and verifies the Oklab
separable result against actual quantize(perturb). Native tests retain the budget and failure matrices.

S35 recipes match the measured large area, center bilinear, and center scale-aware Lanczos3 shapes.
S36 recipes use 769×513 sRGB direct/random fields and 65×49 Oklab blue-noise adaptive fields.
S37 recipes use 65×49 palette-eight adaptive sRGB mixing and 65×49→33×25 nearest/OKLCH palette-four Process.
Required-threaded hosts need hardware concurrency sufficient for at least two initialized workers.
The coordinator must join the S36/S37 selectors before building the ordinary package.

Run after the coordinator supplies its ordinary installed bundle:

```sh
DITHERETTE_BENCH_AUTO_BUNDLE=/absolute/path/bundle-source.json node --test scripts/benchmark-auto-host.test.mjs
```

Chromium and Firefox run once each. WebKit remains excluded under its existing threaded cleanup gate.
The coordinator subsequently authorizes that untimed run after the fixture commit, against the ordinary package built
from `dc81818a`. Set `DITHERETTE_BENCH_AUTO_FIREFOX_EXECUTABLE` to the retained update-disabled
`v1-s35-37-bench/target/rows-trial-03/runtime-source/firefox`. An equivalent optional Chromium executable input is available.
The suite permits ten minutes because observed Firefox scalar field calls take about 25 seconds each.
No Wasm builds, measurements, installs, PRs, or pushes are authorized.

Node 24.19.0 controlled checks pass two tests. Both browser-gated tests skip without supplied bundles.
The controlled adapter test verifies ordinary byte/compiled initialization, role thread policies, exact staged composition,
and disposal through the existing package fixture. Syntax checks pass for all five new/changed scripts.
The old forced-row fixture keeps every assertion; only its asset-copy setup moves to the shared staging helper.
Initial `pnpm exec prettier` stopped on the system Node 24.18 engine mismatch before installation.
Formatting then succeeds with the existing Prettier CLI under pinned Node 24.19.0.
