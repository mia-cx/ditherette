# Releasing ditherette

Publication remains blocked by `release-policy.json`. Its current holds require
initial size-budget review and resolution of the S41 release gates. This document
does not approve either. Human owners also configure the protected `npm-publish`
environment and the npm trusted publisher before any release.

## Prepare and inspect

Use a clean, fixed checkout. Keep that checkout unchanged while retaining its
release evidence. The package owns publication; its build delegates both ordinary
Wasm variants to the crate's pinned build scripts.

```sh
pnpm install --frozen-lockfile
node packages/ditherette/scripts/release.mjs prepare "$PWD/target/release"
node packages/ditherette/scripts/release.mjs verify "$PWD/target/release" v0.1.0
```

The destination must be new. Preparation checks committed source bytes, compiler
pins, public exports, both Wasm variants, and an offline installation of the exact
tarball. It rejects benchmark exports and stale developer payloads.

`release-report.json` records source and artifact SHA-256 digests, actual tool
versions, and every installed file's raw, gzip-9, and Brotli-11 sizes. Tarball raw
size means the uncompressed tar archive. Its gzip size is the actual `.tgz` size;
its Brotli size compresses that same tar archive. Compression figures describe
payloads, not browser memory or measured network latency.

Review each new file and every raw or compressed increase above 10% against
`release-policy.json`. An absent baseline blocks publication. A reviewed baseline
update belongs in source control with its rationale; changing the numbers alone
does not resolve an outstanding hold.

## Tag-driven publication

Decision #36 defines `0.x` as beta without a prerelease suffix. The public npm
package and private Rust crate share an exact version. A `v0.x.x` tag must match
that version. The package publishes to `latest`, with scalar and threaded assets
inside one unscoped MIT browser-ESM package.

The tag workflow builds once, then runs existing native and installed-browser
conformance commands against that tarball. It verifies source, sizes, version,
and recorded holds again immediately before publication. Root imports stay inert;
raw bindings remain private and no public backend selector is added.

Scalar Chromium, Firefox, and WebKit are covered. Threaded Chromium and Firefox
are covered. Pinned WebKit threaded cleanup remains a separate release gate.

Publication uses GitHub OIDC through npm trusted publishing, not a stored npm
token. Bind the trusted publisher to this repository, `package-publish.yml`, and
the protected `npm-publish` environment. Configure required human reviewers in
that environment. npm's trusted-publisher requirements and automatic provenance
are documented in [npm's guide](https://docs.npmjs.com/trusted-publishers/).
GitHub documents [environment protection rules](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments).

The checked tools are Node 24.19.0, pnpm 11.13.0, npm 11.17.0, wasm-pack 0.15.0,
Rust 1.97.0, and the genuine `nightly-2024-08-02` threaded compiler. The frozen
build guard remains authoritative for compiler identity and flags.
