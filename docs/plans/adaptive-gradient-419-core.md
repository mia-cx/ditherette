# Area placement core for issue419

The core/package branch replaces the unshipped directional preview. UI integration, DIRECTION,
trusted freeze approval and delivery belong to the parent/UI owners.

## Contract

`Placement { mode: 'smooth-area-v1', sampleSize, contrastThreshold }`.
Square side is an integer from 4 through 32768 working pixels after resize; tolerance is finite 0..1.
Editor defaults are 64px/0.12. Direct APIs measure source pixels. Samples clip to smaller images.
Frozen legacy Adaptive and Everywhere recipes remain unchanged.

## Selection

Each pixel measures the largest Euclidean Oklab distance to its immediate eight neighbours.
Each square qualifies when its maximum local distance, clipped to 1, is within tolerance.
Select the complete union of all overlapping qualifying footprints. A fixed one-pixel read
halo keeps bold boundary pixels outside every qualifying footprint; it does not grow with size.
A black-to-white ramp over 100px qualifies at 12%; an equally contrasting 1–2px step fails.
Total ramp contrast is unrestricted. Fine-step bounds imply bounds at larger octave distances,
so the maximum-step rule needs no directional fits or repeated pyramid passes.
Sample size requires a coherent square footprint. Tolerance independently bounds perceptual
jump magnitude. The same-width green-to-yellow step qualifies at 25%; green-to-dark-blue fails.
Size does not change the colour metric or substitute a dimensionless fine/broad contrast ratio.
Flat pixels and quantized plateaus qualify with their surrounding area. Low-contrast texture
can qualify. Zero tolerance admits flat areas; one admits the entire image.

The [reference](../../crates/ditherette-wasm/src/spec/smooth_area_v1.md) defines the complete rule.
It has a separate corresponding production copy. The new append-only checkpoint record removes
only the unshipped gradient reference files and registers the area domain.

## Rendering and ownership

Fields retain their normal arithmetic inside selection. Diffusion keeps its ordinary continuous
recurrence and masks only visible output: selected indices equal Everywhere, excluded indices
equal plain matching. Preserved transparent pixels keep the existing sink semantics.

One call-owned allocation stores 12 bytes/pixel plus 12 bytes times max(2 * width, height).
Pixel records hold Oklab, local sharpness, row maxima, square maxima and then footprint coverage.
Two source colour rows preserve live coordinates while sharpness overwrites dead values.
That same storage then holds one monotone queue. A 4096-byte reserve covers scalar records. Source snapshots,
intermediates, retained output, wrappers, allocation headers and actual capacities stay charged.
No global cache or per-worker analysis copy exists. Analysis runs once and releases after the call.
Every analysis/coverage line and render row permits cancellation. Work is O(pixel count).

## Evidence

The original contrast recipe selected hard edges and reconstructed negligible broad-ramp fields.
The first directional preview later selected only 86.79% of the supplied 1024x384 Celeste sky crop
at size 114, with visible strips. Its output leaves holes inside complete qualifying footprints.
The RGB-channel prototype also rejects the lower perceptual contrast green-to-yellow control.
Both failures have actual public-path red proofs in the task cache.

The area proof covers equal-total-contrast ramps and steps, complete overlapping footprints,
quantized plateaus, tolerance monotonicity, transpose/stride/alpha independence, extreme side sizes,
normal Bayer/Sierra texture, source/settings changes, exact budget limits and cancellation recovery.
Original private image bytes and before/after artifacts remain in the task cache outside git.

Developer scalar/threaded builds use separate supported temporary targets under the shared build
lock. They are developer artifacts, not fresh release qualification. Parent owns trusted freeze
approval, final website checks, browser acceptance and deployment.
