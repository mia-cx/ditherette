# S24 literal benchmark artifact

Start at `a23260edec0452fd17c13073636f548b07804230`, the literal quantize implementation checkpoint.
Keep every production, reference, image, and freeze-policy byte unchanged.

## TODOs

- [x] Apply only the common benchmark protocol from `312f301ec1fca0b2ffead469527cfaf83c3d00f3` and a thin literal adapter.
- [x] Verify original production bytes, native callable conformance, and compilation before committing the artifact source.
- [ ] Build the immutable native accepted executable without running measurements.

The literal wrapper calls `prod::quantize::quantize(request)` and registers `baseline:quantize:request:literal`.
The candidate wrapper calls the prepared convenience API with a budget and registers its separate candidate ID.
Both time validation, preparation, owned output allocation, conversion, matching, and result disposal with borrowed source bytes.
The shared forward controls call the unchanged landed converter. Their exact numeric output and byte alpha use the existing verifier.
The baseline never imports later production quantize, public package, resize preparation, or application-cache implementations.

Ten focused Rust tests pass using the candidate worktree's existing target directory.
The tests include 46 full-call quantize cases and five packed-color controls compared exactly with the frozen outputs.
`git diff --quiet a23260ed -- crates/ditherette-wasm/src/prod crates/ditherette-wasm/src/spec crates/ditherette-wasm/src/image tools/spec-freeze` passes.
The separately trusted S18 guard passes native/Wasm isolation and retains the frozen checkpoint and content digest.
Only the quantize subject ID and old one-argument call differ from the candidate's new adapter file.
No operations run under a measurement timer.
