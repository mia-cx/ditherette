# Releases

Implementation PRs run package conformance, website tests, and a production website build. Add a changeset with `pnpm changeset` for each release-worthy change.

On main, [release.yml](../.github/workflows/release.yml) opens or updates `changeset-release/main`. Merging that PR authorizes only the package versions it increases. A website-only release leaves npm unchanged. Changesets patches the website consumer when its npm dependency changes, while keeping separate version numbers.

The merged commit runs conformance again. npm publication builds and tests its exact archive, verifies source identity, size limits, and recorded release holds, then publishes through OIDC. Website deployment uses the tested conformance artifact at that commit. If both versions changed, deployment waits for successful npm publication.

Ordinary pushes, tags, unrelated PR merges, and unchanged versions cannot publish or deploy. Unconsumed changesets defer their affected release until the next release PR. No GitHub releases or release assets are created. Temporary Actions artifacts retain verification evidence.

Release planning runs for every push without a shared queue. Only release-PR updates and each delivery target are serialized. PR updates read the latest main; superseded target versions fail their version check.

## First-release setup

- Set `RELEASE_PR_TOKEN` to a repository-scoped token with contents and pull-request write access. GitHub Actions must be allowed to create pull requests. Using a separate token lets generated release PRs trigger normal CI.
- Configure the `production` GitHub environment with `CLOUDFLARE_API_TOKEN` and the `CLOUDFLARE_ACCOUNT_ID` variable. The token needs permission to deploy the existing Worker and its custom domain. Deployment uses Wrangler's existing `production` environment for `ditherette.mia.cx`.
- Configure the `npm-publish` GitHub environment and npm trusted publishing for `mia-cx/ditherette`, calling workflow `release.yml`, environment `npm-publish`. npm validates the calling workflow when a reusable workflow publishes. No npm token is stored in this pipeline.
- The first candidate uses an approved local npm bootstrap without provenance because trusted publishing needs the package to exist first. Build and verify the exact archive before publishing it under `rc`. Configure trusted publishing before an ordinary release.
- Resolve the remaining S41/WebKit hold in `packages/ditherette/release-policy.json` before ordinary publication. The first candidate's package size has been reviewed, and the 10% size-growth gate stays in place.

No initial changeset accompanies this automation. Merging it does not initiate a release.

## Release candidates

The first npm candidate is `0.1.0-rc.0`, published only under `rc`. It does not change the website version or deploy the website. The stable Changesets workflow refuses automatic publication and deployment while the npm version is an RC. Promotion to the corresponding ordinary version is supported.

The first candidate's reviewed package baseline is 1,358,336 raw bytes, 467,608 gzip bytes, and 315,843 brotli bytes. This is 14.1%, 14.6%, and 13.8% above the earlier baseline after the performance work. The scalar Wasm is 447,703 raw bytes; the optional threaded Wasm is 625,739 raw bytes. Future builds retain the 10% growth gate against this reviewed baseline.

The RC scope is scalar processing by default, with optional threads experimental. The retained WebKit threaded-cleanup limitation remains documented. The ordinary release hold stays in place. The first candidate's one-time local publication omits provenance with Mia's approval; ordinary publication still requires it.

## Reruns

Rerun the original failed workflow. Each release checks that main still has the same version before publication or deployment. A superseded version fails instead of moving production backward. npm reruns skip publication only when registry integrity matches the exact tested archive; mismatches and registry errors fail.

A website rerun redeploys the same tested artifact. If its temporary artifact expired, rerun all jobs to rebuild and test the same commit. Source, package/crate version, provenance, and size checks still apply.

The [Changesets action documentation](https://github.com/changesets/action/tree/ae32849d5ba541f9ae29e40e22a623bc13562f51) defines the pinned v2 inputs. [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/) documents workflow identity and initial setup.
