# S25 metric dispatch candidate

The literal metric baseline is `17b2bb003a48fe3151dc7277e88f0dd55d8bf87b`.
The complete public baseline is `0085972a05a3dbdbbef6d47351d6e37bdd8625d2`.

This candidate selects the metric once per pixel, before scanning palette entries.
Each specialized scan calls the unchanged metric arithmetic in the same order.
The strict comparison retains first-index ties. Preparation and capacity accounting remain unchanged.
Landed conversion kernels and shared helpers remain unchanged.

## Acceptance

- [x] Existing fifteen-mode exact output, tie, and memory tests pass.
- [x] The trusted frozen-spec guard passes.
- [ ] Measure literal and candidate complete calls with fresh alternating pairs.
- [ ] Select only if exact output and the agreed performance gate pass.

Thirteen focused native tests pass across `prod_quantize`, `prod_quantize_allocation`, and `prod_processor_quantize`.
This includes the 375-case matching/palette/alpha matrix and frozen metric bit comparisons.
No measurement has run. This branch is an unselected candidate.
