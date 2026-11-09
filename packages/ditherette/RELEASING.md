# Releasing ditherette

The public package is the unscoped `ditherette` browser ESM package. The npm
package and Rust crate share a version. Scalar processing is the default;
optional threads remain experimental, and release validation does not certify
threaded WebKit. The first approved RC used the separate procedure below.

## Prepare and inspect

Use a clean checkout at the exact commit to release. Keep evidence outside the
repository, and pass a destination that does not yet exist.

```sh
pnpm install --frozen-lockfile
node packages/ditherette/scripts/release.mjs prepare /path/to/evidence
node packages/ditherette/scripts/release.mjs verify /path/to/evidence v0.1.0-rc.0
```

Preparation builds both Wasm variants with pinned tools, packs the public
archive, installs that archive offline, and compares every installed file with
the packed bytes. The report records source and archive digests plus raw, gzip,
and brotli sizes. It rejects stale developer files and build overrides.

The reviewed tar baseline is the measured 1.0.2 archive, with scalar, threaded
and GPU artifacts: 4,019,200 raw bytes, 1,419,563 gzip bytes, and 760,068
brotli bytes. The size gate flags new files and growth above 10% from this
baseline. Review and record any later baseline change.

## First RC bootstrap

Mia approved a one-time local publication of `0.1.0-rc.0` without provenance
because the unregistered package cannot use trusted publishing yet. From the
same verified checkout, publish the exact tested archive under `rc`:

```sh
npm publish /path/to/evidence/ditherette.tgz --access public --tag rc --provenance=false --ignore-scripts
```

Confirm the registry version, `rc` dist tag, archive integrity, and installation
of `ditherette@rc`. This does not release the website or move `latest`. Configure
the npm trusted publisher after the package exists.

## Ordinary releases

Ordinary `0.x.x` releases are the beta channel and use `latest`. Changesets
opens a release PR from `main`. Merging that PR triggers conformance and
publishes only versions that increased. The package release rebuilds and
verifies its exact archive, checks the size budget and recorded holds, then
publishes through GitHub OIDC with npm provenance. The website deploys only
when its own version increases and its release checks pass.

The npm trusted publisher must name `mia-cx/ditherette`, the calling workflow
`release.yml`, and the protected `npm-publish` environment. npm documents the
package prerequisite in [its trusted publishing guide](https://docs.npmjs.com/trusted-publishers/).
Changesets promotion from the RC restores the `latest` package tag and aligns
the Rust crate version.

The checked tools are Node 24.19.0, pnpm 11.13.1, npm 11.17.0, wasm-pack
0.15.0, Rust 1.97.0, and the genuine `nightly-2024-08-02` threaded compiler.
The frozen build guard checks compiler identity and flags.
