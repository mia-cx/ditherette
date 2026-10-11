# Releases

Run `pnpm changeset` with a user-facing change. Select `ditherette` for the npm package or `ditherette-web` for the website. Include that Markdown file in the implementation PR.

**When required:** a PR that touches `src/`, `static/`, `packages/ditherette/src/`, `crates/ditherette-wasm/src/`, or `migrations/` must add a changeset, or carry the `no changeset` label. CI checks this and validates every added changeset (a `patch`, `minor`, or `major` bump on `ditherette` or `ditherette-web`, with a non-empty note).

The release workflow collects changes in `changeset-release/main`. On that PR the **Release validation** check waits for a maintainer's approval in the `release-validation` environment, then runs the full browser conformance suite. Review, approve, and merge that PR to publish changed npm versions, apply D1 migrations, deploy changed website versions, and create `ditherette@<version>` / `ditherette-web@<version>` tags and GitHub releases. Ordinary implementation merges do none of this.

The versions remain independent. A package change also gives its website consumer a patch bump; a website-only change does not bump npm. The private Rust wrapper is excluded. `pnpm release:version` synchronizes the public package version into its Cargo manifest and both consuming lockfiles.

See [release setup and safeguards](../docs/releases.md) before the first release.
