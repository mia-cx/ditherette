# S21 retained measurement and PR assembly

## Branch scope

The S21 branch adds bounded public area/bilinear calls around the restored production kernels.
It includes shared browser recipe registration and protocol fixes, but no S22 production allocation adapters.
Assembly fast-forwarded `056a1324` to `b52d1c8b`, then applied `5d44f386`, `f0ef369c`, and `f9e171ae` without conflicts.
The required rebase onto `fix/v1-restore-landed` at `467542f49ce3f600e5b03aeef574a97be554ae15` preserved the assembled tree exactly.
Historical literal-copy records remain evidence of the superseded replacement decision. Landed kernels stay active.

## Retained trial

The coordinator ran the exclusive shared trial. This PR assembly runs no measurements.
Artifacts remain under `/home/mia/mia-cx/ditherette/.worktrees/v1-resize-integration/target/resize-trial-01`.
Each `s21-<runtime>-results/` directory retains requests, results, stderr, events, prepared identities, verification bundles, and `report.json`.

Native accepted revision is `e64ee3f43d547edd2424c34392e01990c2442c5d`, restored production plus protocol-only fixes.
Measured candidate is integrated runtime `1761705e2c6935544b0232427d48129059d89615`, not the final rebased PR head.
Browser roles use that same integrated build, with TypeScript accepted and package candidate adapters.
The measured tarball SHA-256 is `91729b064dc856fa3cac256369e73f98a41b847aed95f568cf32e2a4a544ac5e`.
The retained tarball bytes and prepared revision records were checked during assembly.

| Runtime | Worker results | Raw samples |
|---|---:|---:|
| Native | 16 | 320 |
| Chromium | 32 | 640 |
| Firefox | 32 | 569 |
| WebKit | 32 | 640 |

Counts come from actual retained result files and sample arrays, not configured maxima.
All eight native accepted/candidate pairs contain exactly equal output bytes and metadata.
Strict frozen-reference gates remain `Incorrect` because both roles retain existing bounded production differences.

| Native case | Accepted ms | Candidate ms | Candidate/accepted |
|---|---:|---:|---:|
| Area reduction latency | 0.6022445 | 0.627319 | 1.041635 |
| Area reduction throughput | 0.5997278 | 0.6275598 | 1.046408 |
| Bilinear reduction latency | 0.9149495 | 0.941359 | 1.028864 |
| Bilinear reduction throughput | 0.8887238 | 0.9105793 | 1.024592 |

Public reduction latency medians below include complete calls. Package/TypeScript outputs are not equivalent; these are diagnostics, not release passes.

| Browser | Area package ms | Area TypeScript ms | Bilinear package ms | Bilinear TypeScript ms |
|---|---:|---:|---:|---:|
| Chromium | 1.4175 | 2.7575 | 1.40 | 0.565 |
| Firefox | 14.21 | 2.38 | 15.14 | 0.36 |
| WebKit | 1.26 | 2.40 | 1.35 | 0.44 |

All four report gates remain `Incorrect`. No exactness policy, reference bytes, or release threshold changed.
No new optimization is selected from these figures. Firefox complete-call overhead and website semantic differences remain separate diagnostic work.

Report SHA-256 values:

- Native: `4ca48b11efd90a93a6d1d91096007e69674b0989cf7fc833165f26841e8c7ebb`
- Chromium: `a3f0c239b154481cf18509b7f29fecef8844ec75f3b97407493f447e3b32dca6`
- Firefox: `b1aed37df2cb6548ef6c91feba664e9607b6c06d1a63e3fd1c36a9382dd3aae7`
- WebKit: `0acaafbb9f5becebbaaab054d40a08ed63f59f3935579d82779d844aabdb384b`

## Final branch validation

- [x] Inspect the full PR diff and confirm no frozen source/image/policy or S22 kernel changes.
- [x] Run 287 native tests, scalar/threaded package builds, six private ABI tests, and the trusted S18 guard.
- [x] Run focused shared protocol checks: 25 Rust library/binary/integration tests and nine fake-clock/transport JavaScript fixtures.
- [x] Install the packed tarball and check Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4. Four reported tests pass.
- [x] Rerun public interface/type checks after correcting the inherited directive placement: 14 pass.

The final branch is ready for its non-draft PR against `fix/v1-restore-landed`; it remains unmerged.

The pinned TypeScript compiler reports an excess `support` property at that property's line.
The inherited fixture placed `@ts-expect-error` above the enclosing multiline declaration, leaving it unused.
Moving the directive to `support` preserves the assertion and changes no production behavior.
Rust formatting and `git diff --check` pass. No benchmarks run during final branch validation.
