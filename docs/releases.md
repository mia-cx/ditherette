# Releases

Implementation PRs run package conformance, website tests, and a production website build. Add a changeset with `pnpm changeset` for each release-worthy change.

On main, [release.yml](../.github/workflows/release.yml) opens or updates `changeset-release/main`. Merging that PR authorizes only the package versions it increases. A website-only release leaves npm unchanged. Changesets patches the website consumer when its npm dependency changes, while keeping separate version numbers.

The merged commit runs conformance again. npm publication builds and tests its exact archive, verifies source identity, size limits, and recorded release holds, then publishes through OIDC. Website deployment uses the tested conformance artifact at that commit. If both versions changed, deployment waits for successful npm publication.

Ordinary pushes, tags, unrelated PR merges, and unchanged versions cannot publish or deploy. Unconsumed changesets defer their affected release until the next release PR. No GitHub releases or release assets are created. Temporary Actions artifacts retain verification evidence.

## First-release setup

- Set `RELEASE_PR_TOKEN` to a repository-scoped token with contents and pull-request write access. GitHub Actions must be allowed to create pull requests. Using a separate token lets generated release PRs trigger normal CI.
- Configure the `production` GitHub environment with `CLOUDFLARE_API_TOKEN` and the `CLOUDFLARE_ACCOUNT_ID` variable. The token needs permission to deploy the existing Worker and its custom domain. Deployment uses Wrangler's existing `production` environment for `ditherette.mia.cx`.
- Configure the `npm-publish` GitHub environment and npm trusted publishing for `mia-cx/ditherette`, calling workflow `release.yml`, environment `npm-publish`. npm validates the calling workflow when a reusable workflow publishes. No npm token is stored in this pipeline.
- `ditherette` was not registered on npm during setup. Establish ownership and the first publication through an explicitly approved bootstrap before relying on trusted publishing. Do not bypass the archive or policy checks for that bootstrap.
- Resolve the existing entries in `packages/ditherette/release-policy.json` through review. They currently hold initial size acceptance and historical S41/WebKit readiness. This automation preserves them and the 10% size-growth gate. Wiring the workflow does not waive them.

No initial changeset accompanies this automation. Merging it does not initiate a release.

## Reruns

Rerun the original failed workflow. Each release checks that main still has the same version before publication or deployment. A superseded version fails instead of moving production backward. npm reruns skip publication only when registry integrity matches the exact tested archive; mismatches and registry errors fail.

A website rerun redeploys the same tested artifact. If its temporary artifact expired, rerun all jobs to rebuild and test the same commit. Source, package/crate version, provenance, and size checks still apply.

The [Changesets action documentation](https://github.com/changesets/action/tree/ae32849d5ba541f9ae29e40e22a623bc13562f51) defines the pinned v2 inputs. [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/) documents workflow identity and initial setup.
