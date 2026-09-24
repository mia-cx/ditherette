# Releases

Run `pnpm changeset` with a user-facing change. Select `ditherette` for the npm package or `ditherette-web` for the website. Include that Markdown file in the implementation PR.

The release workflow collects changes in `changeset-release/main`. Review and merge that PR to publish changed npm versions and deploy changed website versions. Ordinary implementation merges do neither.

The versions remain independent. A package change also gives its website consumer a patch bump; a website-only change does not bump npm. The private Rust wrapper is excluded. `pnpm release:version` synchronizes the public package version into its Cargo manifest and both consuming lockfiles.

See [release setup and safeguards](../docs/releases.md) before the first release.
