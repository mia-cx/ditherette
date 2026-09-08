# #79 Yliluoma row bands

## Summary

Evaluate exact row bands around the landed scalar Yliluoma loop. Share palette preparation and use S35's scheduler.

## Acceptance criteria

- [x] Scalar and row bands agree across worker counts, palettes, Bayer sizes, and adaptive placement in native tests.
- [x] Preflight covers assignment capacity and every live temporary converter; palette preparation stays shared.
- [x] Complete Chromium/Firefox host-worker measurements identify common exact cold winners. S41 retains inconclusive warm controls.
- [x] Record candidate disposition and validated ancestry for the unmerged stacked PR.

## TODOs

- [x] Add a bounded row-band adapter and focused native conformance and memory checks.
- [x] Add bounded complete-call benchmark subjects and public integration instructions.
- [x] Validate joined public mixing dispatch, failure publication, and complete-call memory boundaries.
- [x] Record coordinated browser measurements and candidate disposition.
- [x] Implement conservative mixing recipe-class selection with measured pool fallbacks.
- [x] Join S36's shared indexed callsite and validate automatic complete-call dispatch.

## Notes

- Base S34 is d4531667e1158c2068f30614f40c9d39f8c5313e. Shared S35 executor joins from 20fc297b9bd376883206007cd6a26bb5f139a626.
- Root owns measurement, shared joins, artifact builds, PRs, and tracking. S36 owns `pipeline/indexed.rs`.
- Frozen spec and `image/` remain immutable. The rejected S29 converter candidate remains unselected.
- The adapter keeps literal per-pixel conversion, strict-less-than ties, ordered palette pairs, componentwise mixtures, global Bayer coordinates, and original-source adaptive reads.
- Main-JS Preferred remains scalar; Required fails capability. Pooled calls require a blocking-capable host Worker.
- WebKit threaded pool cleanup remains a visible release gate. Native tests provide no browser cleanup evidence.
- Native release tests `prod_yiluoma` and `prod_yiluoma_row_bands` pass, 12 tests each with and without `--features threads`.
- Validation uses this worktree's fresh `crates/ditherette-wasm/target`; no Wasm build or measurement has run.
- Temporary accounting includes one `Converter` per active worker. The literal matching conversion ends before sequential adaptive neighbor conversions. No mix memo, palette copy, retained color plane, or worker pixel scratch exists.
- `yliluoma_row_band_cases` declares four workloads, each cold and with an explicit final-output cache hit. Native fixture/oracle checks pass (3 tests including reused fixture helpers). Root owns final measurement budgets and forced execution-policy role metadata.
- Existing S34 browser validation only permits host-worker initialization. Root must enable complete-call transport before validating the assembled experiment. This fragment retains explicit host-worker execution; it cannot yet run as an experiment on this branch.
- S36 receives adapter checkpoint e35c9e34. Integration reads `ExecutionPolicy.mixing`, preflights and charges `YliluomaBands::required_bytes`, reserves the adapter, executes with the caller's report closure, then drops/releases it before retention. Scalar remains the default.
- WebKit required-thread measurements remain excluded while its lifecycle cleanup gate fails.
- Shared policy seam joins from f437a18578f786be2ed7cd4cba48f7537db54e32. Commit 7b8ebe0f adapts S36's fef1eafe mixing integration without depending on S36 field code.
- New `prod_processor_yiluoma_bands` tests pass with and without native threads (both enable `bench-subjects` for the private policy setter). They cover frozen complete metadata for both methods, 1/2/4/8 workers, three band heights, policy-independent cache hits, durable outputs, failed input/output copies, joined-worker and Complete callback failures, retry without publication, and one-byte-under preflight rejection with scalar recovery.
- Joined scalar release validation passes 17 tests across `prod_process`, `prod_processor_quantize`, `prod_processor_yiluoma_bands`, `prod_progress`, and `prod_yiluoma_row_bands`. Threaded joined validation passes seven adapter/public tests. Earlier literal scalar-oracle tests pass in both builds.
- Initial candidate checkpoints above kept automatic selection scalar. The measured follow-up below supersedes that hold for the selected recipe classes only.

## Measured follow-up

- [S37 report](../docs/plans/ditherette-v1/s37-row-bands.md) records all twelve cases from Chromium trial 02 and complete Firefox trial 03, source/report identities, exactness, and the unresolved warm controls. Shared S41 validation retains those gaps and WebKit's threaded cleanup gate.
- Shared automatic/explicit policy support joins from bcffc184. Root accepted the report's four recipe-class gates. Smaller/skinny images and unmeasured classes remain scalar. Palette bytes and image contents never select a policy.
- `yiluoma::policy::measured` chooses two workers/4 rows for the small classes. Medium chooses four/16 with at least four pool workers, otherwise two/4. One-worker pools remain scalar; three-worker pools use two/4.
- Focused selector tests pass in release scalar and threaded builds (three each). They exercise measured boundaries, pool fallbacks, unmeasured-class rejection, and palette-content independence.
- Native compiler ownership is this delivery worktree's `target/compiler`. No new Wasm builds or measurements ran here.
- S36's `indexed.rs` callsite joins from 21f7b6da, preserving S35/S36 delivery ancestry. It resolves mixing policy through `Store::row_policy` and keeps existing cache-hit, capacity, progress, and publication boundaries.
- Automatic full-call tests pass with native pools of one, two, three, and four workers. Direct medium and nearest/OKLCH Process calls match frozen indices and metadata with non-fixture adaptive masks. Tests verify measured joined progress, unrelated-stage override preservation, explicit scalar overrides, and policy-independent cache hits.
- Final release checks pass 16 scalar tests (`bench-subjects`) and eight threaded tests (`threads,bench-subjects`), plus the three selector tests per build recorded above. Formatting and the frozen spec/image/guard source diff pass. No compiler or test process remains active at handoff.
- Delivery target ownership returns to root at `target/compiler` (166 MiB, native compiler outputs only). This worktree ran no Wasm build or benchmark.

## Final delivery disposition

Publication branch `delivery/v1-s37-yliluoma` joins S36 PR 127 head `b542bd94a5dbc724de73815ae0008a22985147fd`.
It rebases with merge ancestry preserved onto published `delivery/v1-s36-fields-final`.
The shared plan conflict keeps both the interrupted first sweep and completed trials. Runtime code needs no conflict resolution.
All production, package, and build inputs match the combined tested source `dc81818a`; only untimed fixture scripts differ.

The coordinator's ordinary package passes the trusted frozen guard. Tarball SHA-256 is
`1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
The actual automatic host suite passes 23 checks, including nine cases per Chromium/Firefox engine and medium/Process mixing.
Its exact invocation and browser cleanup are in `.plans/77-79-auto-host.md`. No browser or benchmark repeats during delivery.

All acceptance criteria are complete or explicitly retained as S41 release gates in the measurement report.
S41 owns inconclusive warm controls, automatic-selector overhead follow-up, unmeasured recipe classes, and pinned WebKit threaded cleanup.
The overall paired evidence contains 400 serial workers, 7,215 samples, and 200 exact production comparisons.
Inherited bilinear drift belongs to separate frozen-reference probes, not S37 or same-artifact production drift.
Delivery checks pass `git diff --check`, exact production/build input comparison, and 14 focused Node checks.
The browser-gated test skips without its bundle environment; the previously completed 23-check installed run is not repeated.
