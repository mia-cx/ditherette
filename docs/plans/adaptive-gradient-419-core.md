# Area placement core for issue419

The core/package branch replaces the unshipped directional preview. UI integration, DIRECTION,
trusted freeze approval and delivery belong to the parent/UI owners.

## Contract

`Placement { mode: 'smooth-area-v1', sampleSize, contrastThreshold }`.
Square side is an integer from 4 through 32768 working pixels after resize; tolerance is finite 0..1.
Editor defaults are 64px/0.12. Direct APIs measure source pixels. Samples clip to smaller images.
Frozen legacy Adaptive and Everywhere recipes remain unchanged.

## Selection

Each pixel measures immediate-neighbour Euclidean Oklab distance and twice the residual
between its original colour and a coarse square box mean. Sample size sets the coarse
radius to floor(sampleSize / 2). Each square qualifies when its maximum contrast is within tolerance.
Select the complete union of all overlapping qualifying footprints. The one-pixel fine-contrast
guard keeps bold boundaries excluded. The coarse probe's wider influence can suppress more
area as scale increases, while never activating an edge fringe.
A black-to-white ramp over 100px qualifies at 12%; an equally contrasting 1–2px step fails.
Total ramp contrast is unrestricted. The fine/coarse residual detects concentrated change
even when every immediate step is small. A broad locally linear ramp largely cancels out.
This uses two spatial scales, with no directional fit or whole octave pyramid.
Sample size controls allowed tightness and the coherent footprint. Tolerance bounds perceptual
jump magnitude. The same-width green-to-yellow step qualifies at 25%; green-to-dark-blue fails.
The residual retains amplitude instead of a dimensionless fine/broad contrast ratio.
Flat pixels and quantized plateaus qualify with their surrounding area. Low-contrast texture
can qualify. Zero tolerance admits flat areas; one admits the entire image.

The [reference](../../crates/ditherette-wasm/src/spec/smooth_area_v1.md) defines the complete rule.
It has a separate corresponding production copy. The new append-only checkpoint record removes
only the unshipped gradient reference files and registers the area domain.

## Rendering and ownership

Fields retain their normal arithmetic inside selection. Diffusion keeps its ordinary continuous
recurrence and masks only visible output: selected indices equal Everywhere, excluded indices
equal plain matching. Preserved transparent pixels keep the existing sink semantics.

One call-owned allocation stores 12 bytes/pixel plus 12 bytes times max(3 * width, height).
Pixel records hold Oklab, coarse colour, local contrast, row maxima, square maxima and coverage.
Line records hold original values for running box sums, then three source colour rows,
then the monotone queue. Original source colours convert once again after coarse analysis,
avoiding a second full-image allocation. A 4096-byte reserve covers scalar records. Source snapshots,
intermediates, retained output, wrappers, allocation headers and actual capacities stay charged.
No global cache or per-worker analysis copy exists. Analysis runs once and releases after the call.
Every analysis/coverage line and render row permits cancellation. Work is O(pixel count).

## Evidence

The original contrast recipe selected hard edges and reconstructed negligible broad-ramp fields.
The first directional preview later selected only 86.79% of the supplied 1024x384 Celeste sky crop
at size 114, with visible strips. Its output leaves holes inside complete qualifying footprints.
The RGB-channel prototype also rejects the lower perceptual contrast green-to-yellow control.
Both failures have actual public-path red proofs in the task cache.
The first area rule also accepted an 8-pixel transition at every spatial scale. Its public
counterexample selects 576/576 transition pixels at size 64, where none should qualify.
With the fine/coarse correction, size 64 excludes all 576 compact-transition pixels,
retains all 8256 broad-ramp pixels and retains a lower-contrast compact transition.

The area proof covers equal-total-contrast ramps and steps, complete overlapping footprints,
quantized plateaus, tolerance monotonicity, transpose/stride/alpha independence, extreme side sizes,
normal Bayer/Sierra texture, source/settings changes, exact budget limits and cancellation recovery.
Original private image bytes and before/after artifacts remain in the task cache outside git.

Developer scalar/threaded builds use separate supported temporary targets under the shared build
lock. They are developer artifacts, not fresh release qualification. Parent owns trusted freeze
approval, final website checks, browser acceptance and deployment.
