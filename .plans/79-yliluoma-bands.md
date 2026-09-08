# #79 Yliluoma row bands

## Summary

Evaluate exact row bands around the landed scalar Yliluoma loop. Share palette preparation and use S35's scheduler.

## Acceptance criteria

- [x] Scalar and row bands agree across worker counts, palettes, Bayer sizes, and adaptive placement in native tests.
- [x] Preflight covers assignment capacity and every live temporary converter; palette preparation stays shared.
- [ ] Complete public host-worker measurements decide selection. Scalar remains selected until evidence exists.
- [ ] Record candidate disposition and validated ancestry in the coordinator's unmerged PR.

## TODOs

- [x] Add a bounded row-band adapter and focused native conformance and memory checks.
- [x] Add bounded complete-call benchmark subjects and public integration instructions.
- [x] Validate joined public mixing dispatch, failure publication, and complete-call memory boundaries.
- [ ] Record coordinated browser measurements and candidate disposition.

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
- Remaining TODO belongs to the coordinator's exclusive browser phase. No crossover threshold or speed claim is selected; candidate remains a private override and all defaults remain scalar.
