# Releases

Add a changeset with `pnpm changeset` for each release-worthy change. Changesets manages package versions and changelogs. Website delivery follows verified main commits independently of those versions.

## Website delivery

[CI](../.github/workflows/ci.yml) deploys pushes to `main` and manual CI runs on `main` to https://ditherette.mia.cx. The same run must pass `CI result`, then fresh [release validation](../.github/workflows/release-validation.yml) for its exact commit SHA. This includes package conformance, website types and tests, a production build, and browser tests. Failed, cancelled or skipped validation cannot authorize delivery. PR events, forks and other branches cannot deploy.

The production job checks out that exact SHA, builds the website, applies production D1 migrations, then deploys with Wrangler's existing `production` environment. All website deliveries share one concurrency group. Running deployments finish without cancellation. Pending runs may be replaced or start out of order, so each candidate checks current main before building and immediately before migrations and deployment. A superseded commit fails without touching production.

Website delivery requires no Changesets version PR, npm publication, or cleared npm release holds. The Release workflow no longer deploys the website. CI deployments identify the source commit in GitHub's `production` deployment history; they do not create website version tags or GitHub releases.

## npm releases

On main, [release.yml](../.github/workflows/release.yml) opens or updates `changeset-release/main`. Merging that PR authorizes only the package versions it increases. A website-only release leaves npm unchanged. Changesets patches the website consumer when its npm dependency changes, while keeping separate version numbers.

The merged release runs fresh conformance unless successful release-PR validation covers the identical tree. npm publication builds and tests its exact archive, verifies source identity, size limits, and recorded release holds, then publishes through OIDC. Successful publication creates the npm version tag and GitHub release. Website delivery proceeds separately through CI.

Ordinary pushes, tags, unrelated PR merges, and unchanged npm versions cannot publish to npm. Unconsumed changesets defer their affected npm release until the next release PR. Temporary Actions artifacts retain verification evidence.

Release planning runs for every push without a shared queue. Release-PR updates and npm publication are serialized separately. PR updates read the latest main; superseded npm versions fail their version check.

## First-release setup

- Allow GitHub Actions to create pull requests in repository settings. Changesets uses the built-in `GITHUB_TOKEN`, so release PRs come from `github-actions[bot]`. Use **Approve workflows** on the release PR to start its checks. Release validation has no separate environment approval.
- Keep the existing `production` GitHub environment and `CLOUDFLARE_API_TOKEN`. The token needs Workers Scripts and D1 edit access. CI retains the existing Cloudflare account ID and Wrangler's `production` environment. This change needs no new credential or approval screen.
- Configure the `npm-publish` GitHub environment and npm trusted publishing for `mia-cx/ditherette`, calling workflow `release.yml`. The npm publisher's environment field is optional; if set, use `npm-publish`. npm validates the calling workflow when a reusable workflow publishes. No npm token is stored in this pipeline.
- The first candidate uses an approved local npm bootstrap without provenance because trusted publishing needs the package to exist first. Build and verify the exact archive before publishing it under `rc`. Configure trusted publishing before an ordinary release.
- Resolve the remaining S41/WebKit hold in `packages/ditherette/release-policy.json` before ordinary publication. The first candidate's package size has been reviewed, and the 10% size-growth gate stays in place.

Merging a verified implementation starts website delivery. npm publication still requires a merged Changesets version PR.

## Release candidates

The first npm candidate is `0.1.0-rc.0`, published only under `rc`. npm candidate publication does not deploy the website. The stable Changesets workflow refuses automatic npm publication while the npm version is an RC. Website CI delivery remains independent. Promotion to the corresponding ordinary version is supported.

The first candidate's reviewed package baseline is 1,358,336 raw bytes, 467,608 gzip bytes, and 315,843 brotli bytes. This is 14.1%, 14.6%, and 13.8% above the earlier baseline after the performance work. The scalar Wasm is 447,703 raw bytes; the optional threaded Wasm is 625,739 raw bytes. Future builds retain the 10% growth gate against this reviewed baseline.

The RC scope is scalar processing by default, with optional threads experimental. The retained WebKit threaded-cleanup limitation remains documented. The ordinary release hold stays in place. The first candidate's one-time local publication omits provenance with Mia's approval; ordinary publication still requires it.

## Website retries

For a failed deployment at the current main commit, rerun the failed CI jobs. Successful validation from that run still covers its exact SHA. Rerun all jobs when validation itself failed or needs repeating.

After a workflow fix or a newer main commit, start **CI** on `main`. From the CLI, run `gh workflow run ci.yml --ref main`. This validates and deploys current main using its workflow definition. An old CI rerun cannot deploy after main advances, even when the website version stays the same. To restore earlier behavior, revert it on main and let CI validate the new commit.

Production D1 migrations run before every deploy. If migrations fail, deployment stops. A retry applies outstanding migrations before deploying again.

## npm retries

Rerun the original failed workflow when its workflow definition needs no changes. After a workflow fix, start **Release** on `main` with `retry_release_commit` set to the original release merge's full commit SHA. This uses the updated workflow while building the named release source.

Each npm release checks that main still has the same npm version before publication. A superseded version fails. npm reruns skip publication only when registry integrity matches the exact tested archive; mismatches and registry errors fail.

`retry_release_commit` retries npm publication only. Source, package/crate version, provenance, size checks and recorded release holds still apply.

The [Changesets action documentation](https://github.com/changesets/action/tree/ae32849d5ba541f9ae29e40e22a623bc13562f51) defines the pinned v2 inputs. [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/) documents workflow identity and initial setup.
