# Smooth area placement v1

`{mode: 'smooth-area-v1', sampleSize, contrastThreshold}` replaces the unshipped
directional preview. Frozen legacy Adaptive and Everywhere recipes stay unchanged.
`sampleSize` is an integer square side from 4 through 32768 working pixels, after
process resize. `contrastThreshold` is finite from 0 through 1. Editor defaults
are 64 pixels and 0.12. Larger tolerance can only add selected pixels.

Convert unchanged source RGB8 to D65 Oklab using the inherited f32 conversion.
Compute a coarse colour with a separable symmetric square box mean, using
radius floor(sampleSize / 2) on both axes. At image boundaries, shrink each axis
symmetrically around its centre pixel instead of introducing an artificial edge.
Horizontal means round to f32 before vertical means; sums use f64.
Each pixel's contrast is the maximum of its largest immediate eight-neighbour
Oklab distance and twice its Oklab distance to that coarse colour, clipped to 1.
For every square that fits the image, its contrast is the largest pixel contrast. A square
qualifies when that contrast is at most the tolerance. Mark its entire footprint.
The result is the union of all qualifying, overlapping squares.
For an image smaller than the requested square, clip each side to the image.
The immediate-neighbour test includes an edge one pixel outside the footprint,
so squares ending beside a bold edge cannot admit its boundary pixels. Coarse
means also read up to the selected radius beyond the footprint. Their wider
influence can reject areas as scale increases; it cannot activate an edge fringe.
No per-pixel gradient, direction, fit, flat veto
or edge veto follows whole-square selection.

Sample size sets both spatial tightness and the complete qualifying footprint.
A compact transition can have small one-pixel steps but still differs strongly
from its coarse colour at a larger scale. A broad locally linear perceptual ramp
largely cancels in this residual, even when its total contrast is high.
Twice the residual approaches the complete jump magnitude at a symmetric step.
The immediate-neighbour term retains tiny-scale edge and checkerboard rejection.
Tolerance independently bounds perceptual magnitude, without dividing by total
ramp contrast. A lower-contrast compact transition can therefore still qualify.
This fine/coarse comparison uses two scales rather than a whole octave pyramid.
It tests local curvature and sharpness, without fitting a global affine ramp.
Strong curvature, clustered detail or low-contrast texture can affect qualification.
Distance 1 approximates black-to-white lightness change. Clipping larger distances
makes tolerance 1 unrestricted. The metric distinguishes a bright green-to-yellow
jump (about 0.19) from a same-width bright green-to-dark-blue jump (about 0.73).

Flat squares qualify, including at zero tolerance. This is intentional: a flat
pixel or plateau inside a quantized ramp must receive the surrounding field.
The rule measures regional sharpness, not semantic gradient or texture identity.
Low-contrast texture can qualify. A sample containing a stronger local step fails.
Overlap prevents grid seams and edge-specific activation fringes. At tolerance
one every square qualifies and the output equals Everywhere dithering.

Analysis uses unchanged RGB8, including hidden RGB, independently of alpha,
palette, matching space, field, diffusion state and scan order. Alpha and row
padding do not enter contrast. Separable fields use the resulting binary mask.
Diffusion preserves its ordinary full-image recurrence and only masks visible
indices: selected pixels equal Everywhere; excluded pixels equal plain matching.
Preserved transparent pixels keep the existing sink semantics. Yliluoma chooses
the nearest original colour at excluded pixels.

Coarse colour and local contrast are computed once from the unchanged source. Horizontal then
vertical sliding maxima classify each square. One monotone queue adds and
removes each line index once. It stores exact f32 integers
because validated image sides are at most 32768. Horizontal and vertical interval
unions then mark complete qualifying footprints without overlap counters.
Running box sums add and remove each line entry once. Work is O(pixel count),
independent of sample side and tolerance.

One call-owned allocation contains 12 bytes per pixel plus 12 bytes times
max(3 * width, height). Per-pixel records first hold Oklab, then coarse colour,
local contrast, horizontal extrema, regional contrast and coverage. Line storage
first holds original line values for running box sums. Three source colour rows
then hold unchanged coordinates, converted once again from source RGB8, while
contrast overwrites dead coarse colours. The same storage finally holds the
monotone queue. No additional image plane allocates.
A fixed 4096-byte reserve covers
live scalar records. Every allocation/header and retained source, intermediate,
wrapper and output capacity remains charged before preparation. Analysis runs
once per operation and is released afterward. It is serial in both Wasm variants,
with no per-worker copy or global cache. Cancellation is checked at every scan
line and coverage line; failures publish no partial render or cache entry.
