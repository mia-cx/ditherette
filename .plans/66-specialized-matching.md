# S25 weighted and perceptual matching

Issue #66. Base `0ae8b95f5355b5f311b474034faf2f2bfb686eb5`.

## Ownership and reuse

This worktree owns production metric functions, packed cylindrical integration, quantize registrations, and focused native/private/package fixtures.
The coordinator owns benchmark registration, measurements, joins, and PR preparation.
Builds reuse the idle S24 target at `/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target` exclusively.

Landed `prod/color.rs` already provides Oklch/Cielch conversion and shared Cartesian-to-cylindrical arithmetic. Keep those functions unchanged.
The packed path adds only frozen byte-gray and hue endpoint normalization around that existing conversion.
Specialized metrics and CIEDE2000 are missing production implementations. Copy their frozen functions with mechanical module wiring first.
Keep prepared palette allocation, stable scan order, alpha handling, and complete-result publication from S24.
Spec, image, and policy remain unchanged. No optimization or measurement runs in this task.

## TODOs

- [x] Add literal metric baseline and packed cylindrical wiring; validate all fifteen pairs and commit its exact SHA.
- [ ] Extend private/public quantize tags using existing bounded ownership; validate types, errors, and durable outputs.
- [ ] Build both Wasm variants and validate native/private/package fixtures plus installed tarball in three engines.
- [ ] Record evidence and push a clean handoff for coordinator benchmark work.

## Acceptance

All accepted metric tags select their frozen arithmetic and return exact indices, palette metadata, and warnings.
Known CIEDE2000 vectors, hue seams, neutral colors, ties, and memory failures have focused coverage.
The existing five Euclidean pairs and legacy color APIs retain their behavior.

## Native baseline evidence

Thirteen focused tests pass across `prod_quantize`, `prod_quantize_allocation`, and `prod_processor_quantize`.
The complete-result matrix covers fifteen matching tags, five palettes, and five alpha policies, including truncation and fractional thresholds.
Packed cylindrical output matches frozen f32 bits for 4,096 colors in each space, including every byte gray.
Known CIEDE2000 neutral/hue-boundary vectors and distinct chord/arc scores pass; duplicate ties and exact preparation budgets pass for every tag.
Existing S24 allocation-failure fixtures still pass. The matching tag increases inline matcher ownership, which existing size-based accounting includes.

`prod/quantize/metric.rs` is the frozen file with only `spec` imports changed to `prod`.
`prod/color/lab_ciede2000.rs` copies its frozen function and helpers, omitting the unrelated unavailable image-writer re-export.
Landed color arithmetic remains unchanged; its barrel only exports the new metric module.
