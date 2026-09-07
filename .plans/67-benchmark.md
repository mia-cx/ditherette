# S26 field benchmark registration

Issue #67. Native baseline `b87d965d` starts from accepted all-mode matching `0085972a`, not the rejected S25 dispatch candidate.
Join `07ccc24` includes accepted tooling `bf1b887ca90409b909b742e5eb78f97ab00f5bbd`, including scoped native/public rebuilds.
The join preserves every production byte from `b87d965d`.
Its sole conflict retained S26's existing `rgb8_to_coordinates` addition verbatim.

## TODOs

- [ ] Register exact native inverse, field, placement, source-conversion, and complete Processor adapters with typed identities and focused untimed tests.
- [ ] Extend the existing public protocol and actual package conformance for perturb/separable outputs after S03's public baseline.
- [ ] Declare the fixed 208-worker experiment and conformance fixtures, validate without measurements, and hand off clean checkpoints.

## Fixed proposed measurement scope

Two alternating pairs, 20 samples, 50 ms warmup, 64×48 varied RGBA, strength 0.7, and the S24 ordered palette64.
Sixteen native components plus nine complete recipes in native/Chromium/Firefox/WebKit total 52 cases and 208 serial workers.
Inverse/threshold components use a 250 ms measurement cap.
Source-conversion, placement, and complete calls use a predeclared 10-second cap to retain enough slow baseline samples.
All scopes are single-call latency of one image or component batch, not throughput or cache-hit evidence.

Components are seven original f32 inverse image exports; Bayer2/4/8/16 and random threshold grids;
Oklab radius1 and Oklch radius2 adaptive masks; sRGB and Oklab source-conversion batches including converter construction.
Random seed is `0x12345678`. Adaptive threshold is 10 and softness is 5.
Inverse inputs come from frozen forward conversion outside timing, retaining original byte alpha.
Fields/masks use existing f32 Scores serialization with distinct semantic identities, not rendered-color metrics.
Wide f64 reconstruction remains part of complete perturb, separate from the f32 inverse controls.

Seven complete perturb recipes cover sRGB/Bayer2/everywhere, linearRGB/random/everywhere,
Oklab/Bayer4/adaptive1, Oklch/random/adaptive2, CIELAB/Bayer8/everywhere,
CIELCH/random/adaptive1, and YCbCr/Bayer16/everywhere.
Two complete separable recipes use Bayer4/Oklab/everywhere to sRGB CompuPhase and random/YCbCr/adaptive2 to Oklch hue-arc.
Both use preserve alpha threshold 0.5. Every perturb and matching setting remains in the identity.
Native complete calls retain Processor validation/copies/owned outputs and destruction.
Public conformance compares frozen output and actual `quantize(perturb(...))` outside timing.

No field optimization or measurement is authorized in this task.
The complete native/public baseline must be validated before any call-owned converter candidate.
Use only the assigned S24 benchmark target for Rust checks; preserve its retained `target/s24-*` evidence.
