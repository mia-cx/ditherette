# Smooth gradient placement v1

This append-only domain defines `smooth-gradient-v1`. It does not change the
frozen `adaptive` contrast recipe. Its one public control, `minGradientSize`, is
an integer from 4 through 32768. The editor default is 24. Units are pixels in
the unchanged image entering placement, after resizing for `process`.

Selection reads encoded RGB8, including hidden RGB, independently of palette,
alpha, matching space, field space, scan order and transported diffusion error.
The result is binary. A larger minimum size only removes selected pixels.

The reference scans every horizontal and vertical line. A gradient run follows
coherent channel changes, allowing identical adjacent RGB8 values. Direction locks
after four byte levels of change; reversals up to two byte levels from the running
channel extreme are tolerated as quantization noise. Every channel
step is at most eight byte levels. Short colour intervals end when a channel
changes by four byte levels. Within each interval, all samples must fit a line
through its starting colour within two byte levels per channel. This accounts for
opposite rounding errors in colours quantized to two-byte steps. Consecutive
interval slopes may differ by at most a factor of four and 45 degrees. This
local fit permits broad nonlinear ramps without tightening curvature tolerance
when the minimum spatial size increases.

Runs exclude leading and trailing constant tails. Their end-to-end change must
reach four byte levels in a channel and their support must cover four pixels.
Horizontal and vertical support lengths project onto the stronger gradient
axis, using their RGB slope norms. Equal slopes use the longer support, so
transposing a rectangular image does not change selection. A fixed 3x3 affine residual guard (two bytes
per channel) rejects hard and antialiased boundaries. The guard stays one pixel
wide regardless of the size control. Alpha and row padding never affect it.

This is a local smoothness definition, not semantic segmentation. Strong noise,
channel reversals beyond the quantization tolerance, sharp bends and byte slopes above eight are excluded. Long
smooth monotone portions of a texture can qualify. Signals below four byte
levels cannot qualify. Nonlinear runs use their average RGB slope for spatial
projection; affine ramps have the exact projected raster support length.

`select` receives one caller-owned `[f32;3]` record per working pixel. It stores
horizontal support and slope, then the final mask. It allocates nothing and
uses linear work independent of minimum size. Each line and final guard row
checks cancellation. Production calls charge the record capacity, allocation
header and live selector records before allocation. They release analysis after
the operation and never retain it across calls. Insufficient capacity fails as
the existing memory-limit error, without publishing partial output.

For separable fields, multiply the field by this mask before reconstruction.
For diffusion, every pixel participates in the ordinary Everywhere recurrence,
including its quantization, feedback conversion and error contributions. Selection
only chooses the visible index: selected pixels use that ordinary kernel result;
excluded pixels match their prepared original source colour. This preserves Sierra
and other kernels' texture across byte-quantized mask holes while keeping excluded
edges and flat fills visibly clean. Preserved transparent pixels keep the existing
sink semantics. Everywhere and legacy Adaptive retain their original recurrence. Yliluoma selects
the original nearest palette entry when this mask is zero.
