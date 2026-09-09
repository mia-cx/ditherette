# #85 Integrated validation

Base `c43cea1269fcd666835d41c07d82a1c451604107` joins S41 PR131 at `d09e32df8e06ddddec3d6d374c6f22c280bc4d4b` and S42 PR130 at `db78ca0adf60d1a057d10c2bbe7aabc4c1781374`.

- [x] Validate the fresh joined artifact and repair integration-only fixture wiring.
- [x] Record reused evidence, remaining release blockers, reproduction commands, and cleanup.
- [ ] Root joins the separate stack audit, verifies ancestry, and files the unmerged PR.

Fresh public preparation succeeds from the clean base. Tarball SHA-256 is `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90`; every installed file and compressed size equals S42.

Initial Chromium conformance fails loading the frozen oracle page because the fixture omits S41's indexed-wire module. The oracle staging and package test server both need that dependency. Only these test registrations change.

The first browser invocation uses an incorrect local executable filename and exits before launch. Correcting it exposes the integration failure above. No production or frozen content changes are needed.

After the five-line repair, the existing scalar conformance command passes all 17 checks across Chromium, Firefox, and WebKit. Every engine verifies 367 frozen Yliluoma vectors and 734 untimed adapter calls. Automatic Chromium/Firefox conformance passes 23 checks. Native benchmark protocol/matrix tests pass 49; focused JS/release tests pass 40; package interface/staging/glue pass 42/2/4; website server/Chromium tests pass 68/6. No skips or performance measurements occur.

Fresh scalar memory observations match S42 in all three engines. The artifact stays byte-identical after fixture changes. Package runtime, website runtime, frozen source/guard, Cargo inputs, and build scripts also match the S42 build source. Full native scalar/threaded and threaded lifecycle evidence therefore remain reusable from S42.

No measurements belong to this validation. Reuse S41/S42 evidence only after matching actual files; retain every release blocker. Source, reports, and compact artifact evidence survive cleanup of all owned target directories. PR filing waits for root's final audit join.

The [release-readiness report](85-release-readiness.md) records checks, reuse, blockers, and reproduction commands. Its [artifact manifest](85-artifact.json) distinguishes build source `c43cea12` from validated fixture revision `f331ffcf`.

All validation jobs exited. Cleanup removed the owned root and Wasm `target` trees, about 1.7 GiB. The 1.6 MiB artifact/provenance bundle remains under ignored `benchmark-results/s43-artifact/`. Root owns the separate audit join and PR filing.
