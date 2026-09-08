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
- [ ] Add bounded complete-call benchmark subjects and public integration instructions.
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
