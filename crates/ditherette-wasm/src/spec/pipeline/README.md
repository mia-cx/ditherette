# Complete method references

The public methods have these named Rust compositions:

| Method | Reference |
| --- | --- |
| `resize` | `spec::resize::resize` |
| `perturb` | `spec::pipeline::perturb` |
| `quantize` | `spec::quantize::quantize` |
| `ditherAndQuantize` | `spec::pipeline::dither_and_quantize` |
| `process` | `spec::pipeline::process` |

`pipeline::execute` dispatches the typed request enum and preserves the result kind.
Every request validates before output allocation. Inputs remain borrowed and immutable; results own separate storage.

`process` materializes resized RGBA8, then calls the same dither-family composition as the standalone fused method.
Separable fields materialize clipped, rounded RGBA8 before palette alpha preparation and matching.
None uses direct quantization. Diffusion and Yliluoma use their complete palette-aware references.
Indices, normalized palette bytes, transparency, and ordered warnings are part of both composition equalities.

## Executable control

`processor::Processor` connects real operations to `contract::lifecycle::InstanceModel`.
It starts the call before validation, reports preparation after validation, and releases its state borrow before callbacks.
Recursive processing and disposal therefore return structured reentry errors without disrupting the active operation.

The naive composition reports each materialized stage's start and finish. Counts describe output pixels completed by that stage.
The model immediately reports stage changes and throttles repeated-stage events to one per 50 ms.
Production may offer finer measured work updates; it follows the same state and throttle rules.
The caller supplies a monotonic clock, keeping reference tests independent of real time.

Completion runs only after final bytes and metadata exist. A thrown callback maps to `callback` and discards the result.
An ordinary kernel error aborts without a completion event. Either failure leaves the instance usable.
Successful lifecycle finish is the sole permission to publish newly retained entries.
This naive image processor does not cache computations. The separate cache model defines retained ownership and publication for production.

Disposal is idempotent. Results already returned remain valid, and other instances remain usable.
The reference does not promise Wasm page shrinking or implement a browser allocator.
The package boundary supplies raw-JS shape checks and allocation handling before calling production.
