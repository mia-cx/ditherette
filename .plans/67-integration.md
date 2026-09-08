# S26 baseline integration

Join delivered S25 baseline `bf1b887ca90409b909b742e5eb78f97ab00f5bbd`
with validated S26 field/package baseline `089251287e387cb575e22e8993d8989a371a089d`.
The original literal checkpoint remains `e156cfbf`.

## Integration checks

- [x] Preserve S26's palette-free working-space adapter and shared private quantize parsers during duplicate-history conflicts.
- [x] Verify production, frozen spec, shared image, and complete package files match the validated S26 parent byte-for-byte.
- [x] Pass six native field tests and four bounded-processor tests after the join.
- [x] Pass the trusted frozen-spec guard with the unchanged 106-file digest.
- [x] Join S26 benchmark registration before considering a converter-reuse candidate.
- [x] Register candidate targets and measurement limits before optimization or timing.

The inherited private processor Markdown documentation advances from S25; processing source is unchanged.
S26's existing interface/private/both-build/three-browser evidence remains applicable to these identical implementation bytes.
No new performance evidence or optimization is claimed by this join.

Native benchmark registration `cdbd6e07e98fceb77ebea8bd7b791159b430227d` joins without conflicts.
All four field adapter tests pass in the combined tree. Public benchmark registration completes at `60516c6a`.
The [converter candidate plan](67-converter-reuse.md) fixes its target and single-comparison budget before implementation.

The [completed measurement](67-measurement.md) retains the baseline because required candidate gates remain incomplete.
