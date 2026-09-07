# Full-range BT.601 YCbCr reference

`rgb8_to_ycbcr` uses normalized, gamma-encoded sRGB channels and the BT.601 luma/color-difference coefficients:

```text
Y  = 0.299*R + 0.587*G + 0.114*B
Cb = 0.5 + (B - Y) / 1.772
Cr = 0.5 + (R - Y) / 1.402
```

The packed f32 order is Y, Cb, Cr. Canonical coordinates are `[0,1]`; neutral chroma is exactly 0.5. Ditherette uses full-range coordinates. It does not apply studio-range 16/219 or 128/224 coding, chroma subsampling, or a BT.601 transfer function to browser RGB.

`ycbcr_to_rgb8` reconstructs encoded channels from the affine inverse:

```text
R = Y + 1.402*(Cr - 0.5)
B = Y + 1.772*(Cb - 0.5)
G = Y - (0.114*1.772*(Cb - 0.5) + 0.299*1.402*(Cr - 0.5)) / 0.587
```

The green equation follows by substituting the red/blue equations into luma. It keeps neutral chroma differences exactly zero. Some triplets inside the canonical coordinate cube reconstruct outside the RGB gamut. Clip the reconstructed RGB channels to `[0,1]`, then round `255*channel` to nearest bytes with ties upward. Non-finite coordinates are outside the contract.

The image adapters preserve alpha bytes from the corresponding RGBA8 image, hidden RGB, and row padding as specified in [sRGB reconstruction](srgb.md).

[ITU-R BT.601-7, sections 2.5.1–2.5.2 and Table 1](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.601-7-201103-I!!PDF-E.pdf) provide the coefficients and primary-color signals. Adding 0.5 to the normalized color differences gives Ditherette's full-range convention. Red therefore maps to approximately `[0.299, 0.3312641, 1]`. Neutral `[0.5,0.5,0.5]` reconstructs to `[128,128,128]`.
