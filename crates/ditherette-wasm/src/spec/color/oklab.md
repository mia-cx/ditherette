# oklab color spec

Executable oracle for converting browser RGBA8 image data into this color representation.

Inputs are interpreted as non-HDR sRGB RGBA8. The conversion modules ignore alpha and emit packed f32 color-coordinate images for later matching, quantization, dithering, histograms, and channel adjustments.
