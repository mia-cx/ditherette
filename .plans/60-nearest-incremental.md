# S19 nearest candidate

The accepted implementation is the literal-copy checkpoint `0ede7f6c6f90d6c5d40b169b1dd835f0ac752902`.
This worktree starts from its validated policy join `89b570e0dbb4280352b157bfde5b20c3a7e80a9e`.
The coordinator owns the candidate kernel, nearest conformance tests, and benchmark registration.
Other agents own the private Rust processor and public TypeScript wrapper in separate worktrees.

## Experiment

Nearest copies one pixel per output pixel. The direct reference recomputes a u128 quotient for every coordinate.
Test an allocation-free quotient/remainder recurrence with identical integer rounding for all nine anchors.
An unchanged width permits a logical-row copy, including padded storage and vertical-only resizing.
Keep the literal accepted kernel active until measured evidence supports promotion.

Require exact output across supported formats, independent strides, and all anchors.
Check recurrence arithmetic against the frozen mapper at tiny dimensions and near u32 limits.
No reference or shared-image edits are permitted.

Fresh native pairs compare the copied baseline and this candidate with the frozen oracle.
Use identity, reduction, enlargement, unequal-axis, and small-image cases, each with separate latency and throughput evidence.
Target at least 20% lower median time for nontrivial image cases because the candidate removes per-pixel division.
Require no confirmed per-case slowdown above 10%; fresh measurements supply the actual time values.
Budget four alternating pairs per case and one candidate revision, with one repeat only for inconclusive evidence.
Full public browser-call measurements follow in S20 and remain required even if native results pass.

## TODOs

- [ ] Implement the separate exact candidate and focused conformance checks.
- [ ] Register the candidate, prepare clean binaries, and record the typed experiment before measurement.
- [ ] Drain agents and builds, run one exclusive paired trial, and record the measured outcome without automatic non-exact acceptance.

No benchmark runs during implementation. All PRs remain unmerged.
