# nearest-color quantization

Nearest-color quantization maps each source pixel to a palette index.

The spec accepts precomputed color-coordinate buffers and matching palette coordinates. Color conversion and palette coordinate memoization live outside this module.

Tie rule: stable palette order wins. The nearest scan only updates the best candidate on strictly smaller distance, so exact ties keep the earliest palette entry.
