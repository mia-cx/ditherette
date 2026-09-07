# S17 reference registration subtask

Base: `4f4b48a04c0ccee51222b7e621ff4a566a495613`, `impl/v1-s17-base`.
Branch: `impl/v1-s17-bench`. Main includes these commits in the aggregate S17 PR.

## TODOs

- [x] Extend the existing registry with typed conformance entries and seven f32 color references.
- [x] Join the completed pipeline checkpoint and register all five callable public methods.
- [ ] Verify mode coverage, composition metadata, identity, and missing-role behavior without timings.

Keep concrete request types outside spec's dependencies. S05 owns output records
and comparisons. Legacy resize timings skip conformance-only entries. Settings
serialization retains every semantic stage, including independent perturb and
matching spaces. Reference state remains PreFreeze until S18.

No semantic, production, inventory, ledger, or aggregate-plan edits. No benchmark
measurements or separate PR. Main owns pipeline completion and the final join.

The initial registration checkpoint passes three reference-adapter fixtures,
three legacy/S05 adapter fixtures, and three binary unit tests. Binary/benchmark
compilation passes. Seven color entries round-trip black, white, red, and a mixed
color through their actual f32 inverses with unchanged alpha. Missing production
roles remain incomplete, and differing perturb spaces change settings identity.

Joined main's exact pipeline/strict-variant checkpoint
`b478993bca5dc20bbd9ffef28eee54ca1ee31e54`. The shared registry now contains five
callable public methods and seven color pairs. Five reference fixtures pass,
including all nine anchors, both support policies, all matching/alpha tags,
all field/space combinations, diffusion feedback/scan modes, and Yliluoma sizes.
Tiny independent Bayer bytes verify the RGBA8 intermediate and exact indices;
all composition comparisons include complete palette/transparency/warnings.
