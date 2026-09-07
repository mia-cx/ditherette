# S20 immutable browser assets and worker

Start from validated `86692c27fd525d2d439aefb96b277dfc6d511680`.
The parent owns the slice plan, shared manifests, measurements, and PR.

## TODOs

- [x] Snapshot complete package, compiled TypeScript, script, Playwright, and browser runtime trees.
- [x] Add the owned browser worker, pair CLI preparation, and strict transport/conformance validation.
- [x] Test tampering, malformed output, and child cleanup without measurements; record integration evidence and push.

## Agreed ownership

The protocol agent owns shared browser structs and paired coordinator hooks.
This branch owns browser_assets, paired_browser, main dispatch, and pair CLI changes.
The transport agent owns Node/browser scripts and compilation of the actual TypeScript dependency closure.
The coordinator permits minimal lib/module registration for buildable commits.
No worker compiles, installs, or measures outside an explicitly cleared paired trial.

## Snapshot contract

Read-only snapshots include every regular file under each explicit source directory.
Source links to regular files inside their source root become ordinary copied files.
Escaping/directory links and all snapshot links fail validation.
Changed permissions, missing/added files, and changed content fail validation.
Package metadata must identify installed ditherette and its actual exported entrypoint.
Playwright and playwright-core are snapshotted as sibling packages in a real node_modules directory.
Node remains bound to its canonical path and full hash, rechecked before and after each trial.
The complete browser installation and private libraries are copied and hashed.
Only the reviewed relative WebKit script launcher is accepted; absolute external launchers fail.
Runtime observations must match their declared engine/version/options/isolation identity.
Asset roots are storage locations, not substitutes for content identity.

## Provenance and validation

The builder emits complete source/output manifests and tool versions/full digests.
Preparation checks every tracked input against its committed HEAD blob, independent of Git index flags.
Both accepted and candidate asset revisions must equal their respective worker revisions.
The snapshot retains build-provenance.json in its content identity.

The worker computes frozen output before Node starts and supplies reference_output in a separate retained request.
An untimed mismatch returns complete actual output and zero work counts.
The worker retains S05 raw records/images, rejects the trial, and never fabricates a missing peer role.
Normal trials retain zero/coarse timer samples; malformed identities, metadata, and runtime observations fail.

Real source validation passed for the coordinator's fresh build at `2891f9db6e0ff6004b6d9740433bc78511b3de0d`.
Actual WebKit installation/private-library staging passed without launching a browser.
The trusted S18 guard preserves checkpoint `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Implementation commit is `dc6bcfd627ad07f2afe0027b239a451a2cef335b`.
`cargo test --locked --manifest-path crates/ditherette-bench/Cargo.toml` passes 44 top-level tests.
One child-only fixture stays ignored in the ordinary test listing and runs through its parent test.
Ten focused asset/worker tests cover complete copies, portable identity, tampering, stale/flagged source,
external launchers, malformed response identity, untimed mismatch records, and owned Node cleanup.
`cargo fmt --check --manifest-path crates/ditherette-bench/Cargo.toml` passes.
The malformed-response fixture confirms Node receives termination, records cleanup, and leaves no owned PID.
All owned build/test/preparation processes exited. No browser image measurement ran in this branch.

For integration, read [BROWSER.md](../crates/ditherette-bench/BROWSER.md) before preparing worker/assets from one clean joined revision.
The coordinator owns the full fresh artifact build, actual browser conformance, and any later exclusive measurement window.
