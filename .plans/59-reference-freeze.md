# #59 Freeze the complete reference

## Summary

Freeze the validated S17 reference and its shared image dependency closure.
Enforce content identity, independent compilation, and trusted-base validation.

## Acceptance criteria

- [x] Record the fixed S17 commit, tree IDs, content hashes, inventory, and blue-noise provenance.
- [x] Reject frozen changes and direct or indirect semantic imports in both directions.
- [x] Bind reference dependencies, features, and Rust 1.97 without blocking unrelated S02/S06 integration.
- [x] Demonstrate controlled mutations fail and temporary trees are restored.
- [x] Push implementation for coordinator review and the unmerged S18 PR.

## TODOs

- [x] Capture the immutable checkpoint and implement content validation with a shared-dependency audit.
- [x] Add Rust syntax checks, independent compilation, and resolved dependency validation.
- [x] Add trusted-base CI wiring and focused temporary mutation tests.
- [x] Record final checks and hand off the pushed implementation.

## Notes

- Base/checkpoint: `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, branch `impl/v1-s17-processor`.
- Worktree/branch: `.worktrees/v1-s18-freeze`, `impl/v1-s18-freeze`.
- The coordinator owns `slices.md` and `stack-ledger.md`. Preserve its existing uncommitted slice status update.
- No semantic edits, benchmarks, PR creation, merges, tags, deployment, or publication.
- Read the approved PRD and resolutions in #40. Rechecked SHA-256 cache and thread-pool models in final S17.
- Content validation passed. Frozen SHA-256: `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
- `node tools/spec-freeze/guard.mjs` passed content, compiler, syntax/profile checks, four dependency contexts, native/Wasm spec/prod isolation, and native threaded prod isolation.
- Initial syntax allowlists rejected Serde attributes. Replaced general allowlists with narrow source-injection restrictions; ordinary derives and local macros remain available.
- Seven mutation fixtures pass, including real compiler failures through shared image helpers, JSON `preserve_order`, and nested adapter macro injection.
- An initial JSON fixture updated only its temporary core lockfile. Updating both temporary consumer lockfiles isolates the intended feature mismatch; all fixtures now pass.
- Trusted-base CI is wired with an explicit S18 bootstrap exception. Repository-required checks and policy protections remain administrator-controlled and are not activated by this unmerged branch.

## Final evidence

- `node tools/spec-freeze/guard.mjs`: passed. Checked all 106 frozen files, Rust 1.97.0 compiler identity, syntax/module/profile boundaries, four resolved dependency contexts, native and Wasm isolation for both families, and native threaded production isolation.
- `node --test tools/spec-freeze/guard.test.mjs`: 7 passed, 0 failed, 0 skipped. Expected mutation compiler errors are test evidence, not residual failures.
- `cargo +1.97.0 fmt --manifest-path tools/spec-freeze/syntax/Cargo.toml --check`: passed.
- Root-installed Prettier checked `tools/spec-freeze/*.mjs`, `*.json`, `README.md`, and `.github/workflows/spec-freeze.yml`: passed.
- `git diff --check`: passed.
- S19 preparation join dependency/configuration compatibility passed through the same four-context dependency verifier. The coordinator performs its full trusted-guard integration check after joining S18.
- `git diff --name-only -- crates/ditherette-wasm/src/spec crates/ditherette-wasm/src/image`: empty.
- Spec Git tree: `f443187b7bf84753d10ca2288242a8787d3c2b42`.
- Image Git tree: `138cb82ba25b3250ac25f4ac11ee17d02d09f811`.
- Retention branch: `reference/ditherette-v1`. Validation never reads its tip as the checkpoint.
- No benchmarks ran. All build/test sessions exited. Only the coordinator's slice/ledger edits remain outside the implementation commits.
- CI execution and required-check activation are not claimed. Initial policy bootstrap and future policy extensions require explicit trusted review; compiler/registry infrastructure remains trusted.
- Implementation uses GPT-6-astra at high reasoning in Codex. The coordinator owns PR creation and aggregate ledger completion.
