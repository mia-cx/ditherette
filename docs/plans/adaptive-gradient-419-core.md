# Gradient-only adaptive placement core

Implements the engine and public package portion of [#419](https://github.com/mia-cx/ditherette/issues/419).
The UI owner integrates `src/**`; the parent owns direction, review, freeze approval and delivery.

## Contract

`Placement { mode: 'smooth-gradient-v1', minGradientSize: number }` extends existing recipes.
The integer range is 4..32768; the editor default is 24. Size uses working pixels after resize,
or source pixels in direct APIs. Legacy `adaptive` and Everywhere remain unchanged.
The [reference](../../crates/ditherette-wasm/src/spec/smooth_gradient_v1.md) defines smoothness and its limits.

## Work

- [x] Reproduce edge selection and missing ramp output through actual Bayer and diffusion calls.
- [x] Add the append-only reference, production selector, capacity preflight and diffusion isolation.
- [x] Prove quantized/nonlinear/oblique gradients, flat tails, exact parity, cancellation and bounded native calls.
- [x] Validate actual scalar/threaded package requests, validation and historical frozen conformance.
- [x] Record the append-only freeze extension for parent review and UI integration.

## Diagnosis

Reported Bayer16x16 strength30%, radius6/threshold12/softness12 in sRGB perturbation with Oklab matching
changed 348/528/529 pixels around vertical/horizontal/diagonal steps and none in a 96px smooth ramp.
Floyd-Steinberg changed 378/528/365 step pixels and none in the ramp. Source-only legacy activation
covered 2, 12 and 24 columns at radii 1, 6 and 12 respectively. No resize was required.
Broad-ramp contrast activates only weakly enough that reconstruction rounds the field away.
Legacy diffusion masks outgoing error after matching an already contaminated pixel.

With the new selector, the same hard steps, eight-pixel ramp and flat fill change zero pixels.
The 96px ramp changes 632 Bayer pixels and 629 Floyd-Steinberg pixels.
A 2048px repeated-byte ramp selects 99.32%; the quadratic ramp selects 83.30% at minimum256.
The oblique fixture selects 100%. Short ramps with long flat tails and tested texture select zero.
Every mathematical fixture also preserves selection when transposed. Equal slopes use the longer support.

## Capacity and ownership

The new mode reserves one separate `[f32;3]` record per working pixel before preparing other call buffers.
Actual vector capacity, headers, live legacy placement records and a conservative 4 KiB selector reserve count.
Source snapshots, resized/intermediate images, indices, wrappers and retained caches keep their existing charges.
The selector runs once on unchanged working RGB, then drops its scratch after rendering. It allocates no global cache.
The new mode stays scalar within both Wasm variants; no per-worker selector allocation exists.

For a 70x12 source and 35x6 process output, native exact limits are 41568/43000/42144/33394/33822 bytes
for perturb, fused Bayer, fused diffusion, process Bayer and process diffusion. One byte below fails normally.
Actual public scalar and required-threaded limits agree at 46572/48960/36502 bytes for perturb,
fused Bayer and resized process diffusion. Cancellation and recovery pass at the exact limits.
Source changes recompute analysis; minimum-size changes affect processing identity.

## Validation scope

Synthetic fixtures prove the defect and controls; supplied screenshots are output, not original source images.
They cannot establish exact original-image reproduction. Developer builds use supported task-specific
temporary compiler targets because the root filesystem lacks space. They are not release qualification,
timed performance evidence, GPU hardware evidence or permission to publish.
Reference content, compiler, build configuration and resolved dependencies pass local checks.
The parent owns the exact freeze approval and complete trusted syntax/isolated-compilation guard,
plus final UI evidence, PR integration and delivery. The root-disk capacity exception does not waive those gates.
