# S42 reproducible package and publication preparation

Issue #84. Branch `impl/v1-s42-package` starts at S40 PR #129 head
`96c281180529c3849823736581daebdcdbd8493e`. Decision #36 owns package/version,
toolchain, beta tagging, and publication conventions.

The package owns manifest/docs, release validation, packed artifact size evidence,
and tag-driven publishing. The crate keeps build/test ownership. Root owns S40
conformance commands/workflow; invoke them for the exact release artifact without
changing that workflow. No publishing, tag creation, deployment, credentials,
or benchmark timing belongs to this task.

## TODOs

1. [x] Add the package release contract, ordinary preparation, metadata/docs,
   tag workflow, and focused offline tests as one buildable publication path.
2. [ ] Build both variants locally, verify installed assets, and record per-file
   raw/gzip/Brotli and complete tarball sizes with a proposed initial budget.
3. [ ] Rebase on the supplied S40 base, validate final source/artifact identities,
   and open a non-draft unmerged PR. Report unresolved approvals and idle targets.

Reuse `buildFreshPackage`, `verifyPackageBuildMode`, source-inventory checks,
existing staging, version checking, and installed-browser fixtures. Preserve the
frozen guard and ordinary build configuration. Only this worktree owns its
compiler targets. The release tarball contains no benchmark-feature payload.

## Validation

The four release contract tests pass. They cover package identity, asset layout,
stable beta tags, exact 10% size boundary, unresolved holds, and workflow ordering.
Interface checking before the Wasm build correctly reports missing generated
bindings. Run it after staging both variants. The workflow fetches locked oracle
dependencies before its unchanged offline preparation checks.
