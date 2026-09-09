# S42 package evidence

The ordinary `ditherette@0.1.0` tarball contains 48 files. Publication is not
approved. The initial size budget and S41 release holds remain recorded.

## Fixed artifact

- Source `bdbcb3c812701d50f157a12d7f157138f10b4013`, including S40 parent
  `95706738`. The detached `.worktrees/v1-s42-release-source` checkout remains
  unchanged; later report and budget commits do not replace its source identity.
- Artifact `target/release/ditherette.tgz` in that checkout, SHA-256
  `78a3d5b7321a3dfca8eeb9ee956796a9b6f62d5b2ada94a5f89aada1600c0b90`.
- Source inventory SHA-256
  `2028a4e43c5b72ed230cf501b5a890aa61898b275dcbfd26e2717e0eb7f1150d`.
- [Machine evidence](84-package-sizes.json) records every file's hash and sizes,
  plus the actual Node, pnpm, npm, wasm-pack, and complete compiler identities.

## Proposed size budget

All sizes are bytes. Gzip uses level 9 and Brotli uses quality 11. Tarball raw
size includes tar headers and padding; tarball gzip is the actual `.tgz` length.

| Artifact         |       Raw |    Gzip |  Brotli |
| ---------------- | --------: | ------: | ------: |
| Scalar Wasm      |   380,928 | 167,411 | 134,397 |
| Threaded Wasm    |   548,630 | 202,542 | 160,932 |
| Complete tarball | 1,190,912 | 407,888 | 277,574 |

`packages/ditherette/release-policy.json` records these measured per-file and
tarball values as the proposed initial baseline. Its unresolved review hold
prevents publication. The checker flags new files or any raw/gzip/Brotli increase
above 10%; its exact boundary tests pass. No compression changes runtime code.

For context, the retained S40 ordinary tarball has SHA-256
`1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
Its raw/gzip/Brotli tar sizes are 1,189,888 / 407,467 / 277,034 bytes. The new tgz
grows 421 bytes, about 0.10%. README growth remains below 10% in every metric.
The package manifest exceeds 10% in every metric, at 1,971 / 738 / 619 bytes
versus 1,436 / 580 / 480. Added repository/publication metadata and release commands
explain that growth. This is a review item, not an approved exception.

## Validation

Both ordinary variants were freshly compiled, staged, packed, and installed
offline. Their Wasm export checks exclude `privateExecutionPolicy`; distribution
checks reject benchmark markers, source files, maps, and stale private payloads.
Every installed byte matches the tarball inventory. Public root exports are
unchanged. The source inventory remains identical before and after preparation.

The release contract passes 4 tests; interface/types pass 42, staging passes 2,
and generated scalar/threaded glue passes 4. Version alignment passes. Offline
`npm publish --dry-run --ignore-scripts --offline --access public --tag latest`
passes without publication. Its expected missing-login warning does not affect
dry-run validation. Tag/hold tests confirm the publication path fails closed.

Native and exact-artifact installed-browser conformance results are recorded
after those processes exit. The package uses the existing S40 commands and
frozen-oracle preparation. WebKit threaded cleanup is not silently skipped into
a release approval; that retained engine gate remains unresolved.
