# Smooth area placement v1

`{mode: 'smooth-area-v1', sampleSize, contrastThreshold}` replaces the unshipped
directional preview. Frozen legacy Adaptive and Everywhere recipes stay unchanged.
`sampleSize` is an integer square side from 4 through 32768 working pixels, after
process resize. `contrastThreshold` is finite from 0 through 1. Editor defaults
are 64 pixels and 0.12. Larger tolerance can only add selected pixels.

Convert unchanged source RGB8 to D65 Oklab using the inherited f32 conversion.
First measure the largest Euclidean Oklab distance between each source pixel
and its eight immediate neighbours, clipped to 1. For every square
that fits the image, its contrast is the largest of those local steps. A square
qualifies when that contrast is at most the tolerance. Mark its entire footprint.
The result is the union of all qualifying, overlapping squares.
For an image smaller than the requested square, clip each side to the image.
The one-pixel read halo includes an edge just outside a square's footprint, so
squares ending beside a bold edge cannot admit its boundary pixels. This halo
does not grow with sample size. No per-pixel gradient, direction, fit, flat veto
or edge veto follows whole-square selection.

The controls set independent spatial and contrast limits. Sample size requires
a complete square of smooth neighbourhoods. Contrast tolerance bounds perceptual
jump magnitude, without dividing it by the total ramp contrast. Larger squares
exclude tight spaces between stronger details. They do not change the colour metric.
This distinguishes spatial scales without limiting total colour change. A
black-to-white RGB8 ramp over 100 pixels has small local perceptual steps;
the same change over one or two pixels has much larger steps. Bounding
the finest 2D steps also bounds colour change per pixel at larger octave distances:
along an eight-connected path of length d, Oklab distance is at most d times
the local bound. Extra pyramid passes are unnecessary for this maximum-step rule.
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

Local sharpness is computed once from the unchanged source. Horizontal then
vertical sliding maxima classify each square. One monotone queue adds and
removes each line index once. It stores exact f32 integers
because validated image sides are at most 32768. Horizontal and vertical interval
unions then mark complete qualifying footprints without overlap counters.
Work is O(pixel count), independent of sample side and tolerance.

One call-owned allocation contains 12 bytes per pixel plus 12 bytes times
max(2 * width, height). Per-pixel records first hold Oklab, then local sharpness,
horizontal extrema, regional contrast and coverage. Two line records per column
preserve source colour rows while sharpness overwrites dead coordinates. After
this scan the same storage holds the reusable queue. No additional plane allocates.
A fixed 4096-byte reserve covers
live scalar records. Every allocation/header and retained source, intermediate,
wrapper and output capacity remains charged before preparation. Analysis runs
once per operation and is released afterward. It is serial in both Wasm variants,
with no per-worker copy or global cache. Cancellation is checked at every scan
line and coverage line; failures publish no partial render or cache entry.
