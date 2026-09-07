# S24 literal benchmark artifact

Start at `a23260edec0452fd17c13073636f548b07804230`, the literal quantize implementation checkpoint.
Keep every production, reference, image, and freeze-policy byte unchanged.

## TODOs

- [~] Apply only the common benchmark protocol from `312f301ec1fca0b2ffead469527cfaf83c3d00f3` and a thin literal adapter.
- [ ] Verify original production bytes, native callable conformance, and compilation before committing the artifact source.
- [ ] Build the immutable native accepted executable without running measurements.

The literal wrapper calls `prod::quantize::quantize(request)` and registers `baseline:quantize:request:literal`.
The candidate wrapper calls the prepared convenience API with a budget and registers its separate candidate ID.
Both time validation, preparation, owned output allocation, conversion, matching, and result disposal with borrowed source bytes.
The shared forward controls call the unchanged landed converter. Their exact numeric output and byte alpha use the existing verifier.
The baseline never imports later production quantize, public package, resize preparation, or application-cache implementations.
