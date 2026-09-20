# S43 published stack ancestry audit

The integration tip contains all 43 published implementation PR heads.
This covers S01–S42 and the separate landed-kernel restoration PR109.
No prerequisite ancestry gap appears.

Audited tip is `c43cea1269fcd666835d41c07d82a1c451604107`.
GitHub metadata is observed at `2026-09-09T18:15:46.273Z`.
This report audits ancestry and PR state. It does not establish release readiness or integrated test results.

## Final join and PR state

The final join has exactly two parents.

| Dependency | PR | Published head and merge parent |
| --- | --- | --- |
| S41 / issue #83 | [#131](https://github.com/mia-cx/ditherette/pull/131) | `d09e32df8e06ddddec3d6d374c6f22c280bc4d4b` |
| S42 / issue #84 | [#130](https://github.com/mia-cx/ditherette/pull/130) | `db78ca0adf60d1a057d10c2bbe7aabc4c1781374` |

All 43 PRs are open, non-draft, and unmerged, with auto-merge disabled.
Their 43 associated implementation/correction issues remain open.
All current published base SHAs are ancestors of their respective child heads and the integration tip.

The audit checks 73 original slice prerequisite edges.
Each prerequisite's current published head is present in the child's current PR base, child head, and final tip.
PR109's immediate S20 base adds one corrective dependency edge, which also passes.
The final S43 join directly contains its two required published heads.

## Published inventory

Heads and bases below come from live GitHub PR metadata, not the older ledger's delivery snapshots.
Each base SHA is the current published branch tip, not necessarily the original validated start point.
The table abbreviates SHAs to twelve characters. The [machine report](85-stack.json) retains every full SHA and ancestry result.

Dependencies follow original issue bodies and the tracked slice index.
PR109 is additional corrective work for issue #108; its listed S20 dependency comes from its immediate PR base.
S19's original PR stays recorded alongside its correction.

| Slice / issue | PR | Published head | Current PR base branch and SHA | Current prerequisite heads |
| --- | --- | --- | --- | --- |
| S01 / #42 | [#75](https://github.com/mia-cx/ditherette/pull/75) | `a213effed4b4` | `main` @ `edc87f5da2b6` | None |
| S02 / #43 | [#88](https://github.com/mia-cx/ditherette/pull/88) | `bc110d91d441` | `impl/v1-s01-anchor` @ `a213effed4b4` | S01 `a213effed4b4` |
| S03 / #44 | [#89](https://github.com/mia-cx/ditherette/pull/89) | `fa3007fffc9e` | `impl/v1-s01-anchor` @ `a213effed4b4` | S01 `a213effed4b4` |
| S04 / #45 | [#90](https://github.com/mia-cx/ditherette/pull/90) | `b5ed4d250cbd` | `impl/v1-s01-anchor` @ `a213effed4b4` | S01 `a213effed4b4` |
| S05 / #46 | [#96](https://github.com/mia-cx/ditherette/pull/96) | `8c05906cb9cf` | `impl/v1-s05-base` @ `61224131f9bb` | S03 `fa3007fffc9e`, S04 `b5ed4d250cbd` |
| S06 / #47 | [#102](https://github.com/mia-cx/ditherette/pull/102) | `a65f53e24b88` | `impl/v1-s05-verification` @ `8c05906cb9cf` | S05 `8c05906cb9cf` |
| S07 / #48 | [#91](https://github.com/mia-cx/ditherette/pull/91) | `7ef52bd2bcae` | `impl/v1-s03-contracts` @ `fa3007fffc9e` | S03 `fa3007fffc9e` |
| S08 / #49 | [#92](https://github.com/mia-cx/ditherette/pull/92) | `d5e2d9761481` | `impl/v1-s03-contracts` @ `fa3007fffc9e` | S03 `fa3007fffc9e` |
| S09 / #50 | [#94](https://github.com/mia-cx/ditherette/pull/94) | `d8bcdcdbe8f8` | `impl/v1-s03-contracts` @ `fa3007fffc9e` | S03 `fa3007fffc9e` |
| S10 / #51 | [#97](https://github.com/mia-cx/ditherette/pull/97) | `47712a500c40` | `impl/v1-s10-base` @ `af9c0d99dd14` | S07 `7ef52bd2bcae`, S08 `d5e2d9761481`, S09 `d8bcdcdbe8f8` |
| S11 / #52 | [#93](https://github.com/mia-cx/ditherette/pull/93) | `bb9421cbecde` | `impl/v1-s03-contracts` @ `fa3007fffc9e` | S03 `fa3007fffc9e` |
| S12 / #53 | [#95](https://github.com/mia-cx/ditherette/pull/95) | `01df66826e53` | `impl/v1-s12-base` @ `bdf79a606c94` | S07 `7ef52bd2bcae`, S08 `d5e2d9761481` |
| S13 / #54 | [#98](https://github.com/mia-cx/ditherette/pull/98) | `2a2b0f2de259` | `impl/v1-s13-base` @ `bfa3d42b79dc` | S12 `01df66826e53`, S09 `d8bcdcdbe8f8` |
| S14 / #55 | [#99](https://github.com/mia-cx/ditherette/pull/99) | `769050190114` | `impl/v1-s13-base` @ `bfa3d42b79dc` | S12 `01df66826e53`, S09 `d8bcdcdbe8f8` |
| S15 / #56 | [#101](https://github.com/mia-cx/ditherette/pull/101) | `cfec5d9b7ab9` | `impl/v1-s15-base` @ `578d677822d5` | S10 `47712a500c40`, S12 `01df66826e53` |
| S16 / #57 | [#100](https://github.com/mia-cx/ditherette/pull/100) | `d58355e617bf` | `impl/v1-s15-base` @ `578d677822d5` | S10 `47712a500c40`, S12 `01df66826e53` |
| S17 / #58 | [#103](https://github.com/mia-cx/ditherette/pull/103) | `cef2b60a635f` | `impl/v1-s17-base` @ `4f4b48a04c0c` | S05 `8c05906cb9cf`, S10 `47712a500c40`, S11 `bb9421cbecde`, S13 `2a2b0f2de259`, S14 `769050190114`, S15 `cfec5d9b7ab9`, S16 `d58355e617bf` |
| S18 / #59 | [#104](https://github.com/mia-cx/ditherette/pull/104) | `662d6483c09d` | `impl/v1-s17-processor` @ `cef2b60a635f` | S17 `cef2b60a635f` |
| S19 / #60 | [#105](https://github.com/mia-cx/ditherette/pull/105) | `7de86d799a25` | `impl/v1-s19-base` @ `1bd175127f92` | S02 `bc110d91d441`, S06 `a65f53e24b88`, S18 `662d6483c09d` |
| S20 / #61 | [#107](https://github.com/mia-cx/ditherette/pull/107) | `e19e12c21960` | `impl/v1-s19-integration` @ `7de86d799a25` | S19 `7de86d799a25` |
| S21 / #62 | [#110](https://github.com/mia-cx/ditherette/pull/110) | `2d5412664ccd` | `fix/v1-restore-landed` @ `467542f49ce3` | S19 `7de86d799a25`, S20 `e19e12c21960` |
| S22 / #63 | [#111](https://github.com/mia-cx/ditherette/pull/111) | `9eecc670d9ff` | `impl/v1-s21-area-bilinear` @ `2d5412664ccd` | S19 `7de86d799a25`, S20 `e19e12c21960` |
| S23 / #64 | [#112](https://github.com/mia-cx/ditherette/pull/112) | `cd7a0d298755` | `impl/v1-s22-convolution` @ `9eecc670d9ff` | S21 `2d5412664ccd` |
| S24 / #65 | [#113](https://github.com/mia-cx/ditherette/pull/113) | `f8a2cc11dc42` | `impl/v1-s22-convolution` @ `9eecc670d9ff` | S19 `7de86d799a25`, S20 `e19e12c21960` |
| S25 / #66 | [#114](https://github.com/mia-cx/ditherette/pull/114) | `af8259ac7662` | `impl/v1-s24-bench` @ `f8a2cc11dc42` | S24 `f8a2cc11dc42` |
| S26 / #67 | [#115](https://github.com/mia-cx/ditherette/pull/115) | `59036e1aef87` | `impl/v1-s25-delivery` @ `af8259ac7662` | S25 `af8259ac7662` |
| S27 / #68 | [#116](https://github.com/mia-cx/ditherette/pull/116) | `c9666288cbe0` | `impl/v1-s26-integration` @ `59036e1aef87` | S26 `59036e1aef87` |
| S28 / #69 | [#118](https://github.com/mia-cx/ditherette/pull/118) | `f4dfef7401d5` | `impl/v1-s27-bench` @ `c9666288cbe0` | S25 `af8259ac7662`, S26 `59036e1aef87` |
| S29 / #70 | [#117](https://github.com/mia-cx/ditherette/pull/117) | `6eb9e00fd319` | `impl/v1-s27-bench` @ `c9666288cbe0` | S25 `af8259ac7662`, S26 `59036e1aef87` |
| S30 / #71 | [#119](https://github.com/mia-cx/ditherette/pull/119) | `f408bc99a80d` | `impl/v1-s30-base` @ `af59df116193` | S23 `cd7a0d298755`, S22 `9eecc670d9ff`, S25 `af8259ac7662`, S27 `c9666288cbe0`, S28 `f4dfef7401d5`, S29 `6eb9e00fd319` |
| S31 / #72 | [#121](https://github.com/mia-cx/ditherette/pull/121) | `59b1fe3acdbe` | `impl/v1-s30-process` @ `f408bc99a80d` | S30 `f408bc99a80d` |
| S32 / #73 | [#122](https://github.com/mia-cx/ditherette/pull/122) | `127a0428a0bf` | `impl/v1-s31-preparation` @ `59b1fe3acdbe` | S31 `59b1fe3acdbe` |
| S33 / #74 | [#123](https://github.com/mia-cx/ditherette/pull/123) | `b2ca677ed992` | `impl/v1-s32-stages` @ `127a0428a0bf` | S32 `127a0428a0bf` |
| S34 / #76 | [#124](https://github.com/mia-cx/ditherette/pull/124) | `d4531667e115` | `impl/v1-s33-progress` @ `b2ca677ed992` | S33 `b2ca677ed992` |
| S35 / #77 | [#126](https://github.com/mia-cx/ditherette/pull/126) | `6bbe113b99a0` | `impl/v1-s34-threads` @ `d4531667e115` | S34 `d4531667e115` |
| S36 / #78 | [#127](https://github.com/mia-cx/ditherette/pull/127) | `b542bd94a5db` | `delivery/v1-s35-resize` @ `6bbe113b99a0` | S34 `d4531667e115` |
| S37 / #79 | [#128](https://github.com/mia-cx/ditherette/pull/128) | `91b114ba6105` | `delivery/v1-s36-fields-final` @ `b542bd94a5db` | S34 `d4531667e115` |
| S38 / #80 | [#120](https://github.com/mia-cx/ditherette/pull/120) | `0305456bc252` | `impl/v1-s30-process` @ `f408bc99a80d` | S30 `f408bc99a80d` |
| S39 / #81 | [#125](https://github.com/mia-cx/ditherette/pull/125) | `5938b248506a` | `impl/v1-s39-base` @ `2a0237680fa2` | S38 `0305456bc252`, S34 `d4531667e115` |
| S40 / #82 | [#129](https://github.com/mia-cx/ditherette/pull/129) | `95706738e4f3` | `impl/v1-s40-base` @ `5fccb9e6a51c` | S35 `6bbe113b99a0`, S36 `b542bd94a5db`, S37 `91b114ba6105`, S39 `5938b248506a` |
| S41 / #83 | [#131](https://github.com/mia-cx/ditherette/pull/131) | `d09e32df8e06` | `impl/v1-s40-conformance` @ `95706738e4f3` | S40 `95706738e4f3`, S20 `e19e12c21960` |
| S42 / #84 | [#130](https://github.com/mia-cx/ditherette/pull/130) | `db78ca0adf60` | `impl/v1-s40-conformance` @ `95706738e4f3` | S34 `d4531667e115`, S40 `95706738e4f3` |
| restoration / #108 | [#109](https://github.com/mia-cx/ditherette/pull/109) | `467542f49ce3` | `impl/v1-s20-browser-bench` @ `e19e12c21960` | S20 `e19e12c21960` |

All table entries pass the head/base/dependency ancestry checks.
Every PR targets its published immediate parent or dedicated integration base.
Sibling delivery branches may contain additional prerequisites transitively; the table keeps the original declared dependency list.

## Dedicated dependency joins

Ten dedicated base branches serve twelve slice PRs.
S13/S14 share one join; S15/S16 share another.
Each listed base contains every current published prerequisite head for its dependent slices.

The merge list contains commits reachable from the base after excluding prerequisite histories.
It records actual ancestry; a base tip can be a later documentation or integration-correction commit.
Full merge SHAs, parents, and subjects remain in the machine report.

| Dependent slices | Dedicated base branch | Current base tip | Join merge commits |
| --- | --- | --- | --- |
| S05 | `impl/v1-s05-base` | `61224131f9bb` | `61224131f9bb` |
| S10 | `impl/v1-s10-base` | `af9c0d99dd14` | `af9c0d99dd14`, `bdf79a606c94` |
| S12 | `impl/v1-s12-base` | `bdf79a606c94` | `bdf79a606c94` |
| S13, S14 | `impl/v1-s13-base` | `bfa3d42b79dc` | `bfa3d42b79dc` |
| S15, S16 | `impl/v1-s15-base` | `578d677822d5` | `578d677822d5` |
| S17 | `impl/v1-s17-base` | `4f4b48a04c0c` | `5c0c471d7878`, `606134307105`, `c0c56dfd8131`, `5bcb147801d3`, `5b9719549004`, `8fdc48c032e6` |
| S19 | `impl/v1-s19-base` | `1bd175127f92` | `1dd8128a8532`, `1f7e7a68803f`, `363324c43556`, `6131ad1b53bc`, `ad2410b481d7`, `04f419d433b1` |
| S30 | `impl/v1-s30-base` | `af59df116193` | `22b6dd78a6e5`, `331bf6b4f3f3`, `246297c94dda`, `9a9cdfb78c66`, `6d1e3827e307`, `82a7e3e9e802`, `12c4ba7d79c7`, `0211fcae0e89`, `d14b166f4171`, `3bdf8f5ab6de`, `9432ddde43ed` |
| S39 | `impl/v1-s39-base` | `2a0237680fa2` | `2a0237680fa2`, `72e7b883212f` |
| S40 | `impl/v1-s40-base` | `5fccb9e6a51c` | `5fccb9e6a51c`, `19240e706c7b` |

The S40 base joins the thread-delivery chain with website cancellation/fallback.
Its ancestry includes S35, S36, S37, and S39 at their current published heads.
The final S43 merge then joins S41 evidence and S42 package preparation.
These joins preserve the open PR stack; none merges a PR on GitHub.

## Tracking differences and held work

The tracked slice index in this integration tree is stale from S28 onward.
It still marks S28/S29 in progress and S30–S43 not started.
The root progress mirror records S28–S40 and S42 as ready, S41's PR open with release gates held, and S43 in progress.
Sixteen rows differ. The machine report preserves both statuses and hashes of both files.

The original prerequisites agree between the live issue bodies and tracked index.
The audit therefore uses that dependency contract while obtaining every current head and base from GitHub.
Neither stale progress text nor historical ledger SHAs determine the observed PR identities.
The coordinator owns any progress-table or ledger refresh; this audit changes neither.

S44 issue #86 and S45 issue #87 remain open and not started.
Both tracked and root progress tables record that state.
A scan of all 55 repository PRs finds no S44/S45 branch or deliverable-title match.
Local branches and published remote heads also have no S44/S45 branch.
This checks visible repository state under the agreed naming convention; it does not infer hidden work.

S44 still depends on S43 and prepares a separate held rollout PR.
S45 still depends on S44 and prepares a separate held retirement PR.
Their preparation and activation gates remain distinct.
Mia's stability and rollout acceptance remains required before activation.

## Reproduction and limits

Read current PR identities, then check each recorded SHA against the exact integration tip.

```sh
gh pr list --state open --limit 100 \
  --json number,headRefName,headRefOid,baseRefName,baseRefOid,state,isDraft,mergedAt,autoMergeRequest
git merge-base --is-ancestor PUBLISHED_HEAD c43cea1269fcd666835d41c07d82a1c451604107
git merge-base --is-ancestor PUBLISHED_BASE CHILD_HEAD
git merge-base --is-ancestor PREREQUISITE_HEAD CHILD_HEAD
git merge-base --is-ancestor PREREQUISITE_HEAD PUBLISHED_BASE
```

GitHub's GraphQL query returns all 43 open PRs with no additional page.
The issue query returns all 76 issue records with no additional page.
The machine report records exact inputs, dependency joins, source-document hashes, and absence checks.

This bounded audit runs read-only Git and GitHub checks.
It runs no build, test, browser, benchmark, publication, deployment, or GitHub mutation.
Integrated validation and the release blockers remain separate S43 evidence.
