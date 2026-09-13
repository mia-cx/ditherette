# sRGB color reference

`rgb8_to_srgb` divides each encoded RGB byte by 255. It returns a packed f32 triplet in R, G, B order. The canonical coordinate domain is `[0,1]` per channel; no transfer function runs.

`srgb_to_rgb8` accepts finite coordinates. It clips each channel to `[0,1]`, multiplies by 255, and rounds to the nearest integer. Half-way values round upward. Thus encoded 0.5 becomes byte 128, while values below zero and above one saturate to 0 and 255. Non-finite internal coordinates are outside this reference contract.

The image adapters use existing typed image views and preserve row padding. Forward conversion leaves alpha in the source RGBA8 image. Inverse conversion reads alpha from the corresponding RGBA8 image and copies its byte unchanged. It neither premultiplies RGB nor discards hidden RGB at alpha zero. All three inverse views must have equal dimensions.

The encoded coordinate interpretation follows [W3C CSS Color 4, sRGB](https://www.w3.org/TR/css-color-4/#predefined-sRGB). Byte clipping and rounding are Ditherette's reconstruction rule, consistent with its existing resize reference.
