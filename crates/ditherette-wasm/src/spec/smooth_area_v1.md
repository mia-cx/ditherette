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
qualifies when that contrast is at most the tolerance. Its centre seeds a smooth
region. Overlapping squares supply dense centres rather than a rectangular mask.
For an image smaller than the requested square, clip each side to the image.
The immediate-neighbour test includes an edge one pixel outside the footprint.
Coarse means also read up to the selected radius beyond the footprint. This
classifies trusted regional interiors; the reconstruction below repairs the
rejected gradient side beside lineart without crossing the source boundary.

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
At tolerance one every square qualifies and the output equals Everywhere dithering.

## Region shape and density

Remove eight-connected centre components smaller than ceil(clipped square area / 16).
This suppresses isolated qualifying patches inside brush texture. There is no
global component cache, palette dependency or directional gradient fit.

The accepted fine/coarse contrast also defines where reconstruction may travel.
Pixels within tolerance are traversable. A narrow source boundary can cast a
coarse rejection shadow onto an otherwise smooth background. Identify this
shadow with a fixed source-only witness, independent of tolerance. Measure the
largest immediate neighbour distance and the Oklab axis range over that 3x3
neighbourhood plus nine probes at axis offsets -4, 0, +4. A nonzero fine distance
at least half the range norm is a concentrated boundary witness. The area within
sampleSize / 2 of a witness is also traversable. This support test alone never
selects a pixel. Actual source-colour barriers still prevent crossing lineart.

Two eight-neighbour chamfer sweeps measure distance to this traversable domain's
rejection sites. With r = max(1, sampleSize / 2), its soft weight is
smoothstep(clamp(distance / r, 0, 1)). Image borders are not rejection sites.
Thus genuine texture boundaries soften without eroding the gradient side of
hard lineart. Distances use axial cost 1 and diagonal cost sqrt(2).

Reconstruct from retained centres with four bounded eight-neighbour chamfer
sweeps, in the four combinations of row/column scan direction. A pixel outside
the traversable domain is a barrier. A clipped Oklab jump greater than tolerance
is also a barrier. Diagonal movement requires both connecting axial jumps to
pass, so it cannot cut across a thin line. At tolerance zero only identical
colours connect. Otherwise, a step of length d costs
`d * (1 + sampleSize * (jump / (d * tolerance))^2)`.
Local variation therefore limits extension into texture. Coverage is the soft
domain weight times `smoothstep(clamp((2 * sampleSize - distance) / sampleSize, 0, 1))`.
Full coverage persists within one sample size of trusted centres, then fades.
This bounded chamfer approximation has rounded contours without a radius-squared
search. It can conservatively miss a winding route that needs further sweeps.
Low-contrast connected texture can still qualify; the test has no semantic
knowledge of paint, clothing or lineart. Increasing tolerance only adds coverage:
the source witness is fixed, traversability and seeds grow, and path costs fall.

Density uses the inherited 32x32 blue-noise rank field, with coordinates transposed
and offset by (11, 17) to separate coverage from the selected field. A pixel is
selected when coverage exceeds that field value plus 0.5. Zero remains plain;
one retains the complete normal pixel. Fractional coverage never blends colours.

Analysis uses unchanged RGB8, including hidden RGB, independently of alpha,
palette, matching space, field, diffusion state and scan order. Alpha and row
padding do not enter contrast. Separable fields gate their complete ordinary
perturbation with the density decision, retaining phase and interior texture.
Diffusion preserves its ordinary full-image recurrence and only masks visible
indices: selected pixels equal Everywhere; excluded pixels equal plain matching.
Preserved transparent pixels keep the existing sink semantics. Yliluoma chooses
the nearest original colour at excluded pixels.

Coarse colour and local contrast are computed once from the unchanged source. Horizontal then
vertical sliding maxima classify each square. One monotone queue adds and
removes each line index once. It stores exact f32 integers
because validated image sides are at most 32768.
Running box sums add and remove each line entry once. Work is O(pixel count),
independent of sample side and tolerance.

One call-owned allocation contains 12 bytes per pixel plus 12 bytes times
max(3 * width, height). Per-pixel records first hold Oklab, then coarse colour,
local contrast, horizontal extrema, regional contrast and coverage. Line storage
first holds original line values for running box sums. Three source colour rows
then hold unchanged coordinates, converted again from source RGB8, while
contrast overwrites dead coarse colours. The same storage holds the monotone
queue and source rows for each reconstruction sweep. The nonnegative Oklab L
sign carries the boundary witness through coarse means without changing their
values. A dead per-pixel lane stores component-queue indices as u32 bits, never
through floating arithmetic. Validated pixel counts are at most 2^30, so these
bit patterns are finite. No additional image plane allocates.
A fixed 4096-byte reserve covers
live scalar records. Every allocation/header and retained source, intermediate,
wrapper and output capacity remains charged before preparation. Analysis runs
once per operation and is released afterward. It is serial in both Wasm variants,
with no per-worker copy or global cache. Cancellation is checked at every scan
line, reconstruction row and each width-sized component traversal/tag batch;
failures publish no partial render or cache entry.
