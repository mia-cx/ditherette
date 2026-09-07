# S23 and S24 validation join

Validate coordinator merge `d14b166f4171b028c8b194a533b16cfdccf7b10c` without measurements.
Its parents are delivered trilinear integration `b9b7f96b` and S24 `884a8868f52e82ae579835eee173c6385a9c9ca2`.
The coordinator owns progress and existing PRs. This task owns validation evidence and any necessary focused merge corrections only.

## TODOs

- [~] Review shared merge resolutions and verify both kernel families retain their delivered bytes.
- [ ] Run native implementation, benchmark protocol/adapter, formatting, Wasm, and trusted freeze checks.
- [ ] Build both package variants and validate interface/types, private ABI, and installed tarball in three engines.
- [ ] Check the actual benchmark adapter against the new tarball without timers; commit evidence and drain.

Reuse this coordinator's existing Wasm target and the exclusive S24-bench benchmark target.
Initial free space is 3.4 GiB. The old S23-trilinear debug target is 1.8 GiB and remains untouched unless space requires cleanup.
Preserve every immutable S23/S24 artifact and result directory. No benchmark process or measurement is authorized.
