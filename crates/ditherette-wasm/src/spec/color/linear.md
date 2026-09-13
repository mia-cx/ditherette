# Linear sRGB color reference

`rgb8_to_linear_rgb` normalizes RGB bytes to encoded coordinates `s = byte / 255`, then applies the sRGB decoding equation:

```text
l = s / 12.92                         when s <= 0.04045
l = ((s + 0.055) / 1.055)^2.4         otherwise
```

The output is a packed f32 R, G, B triplet with canonical channel domain `[0,1]`. `linear_rgb_to_rgb8` applies the inverse equation:

```text
s = 12.92 * l                        when l <= 0.0031308
s = 1.055 * l^(1/2.4) - 0.055        otherwise
```

Reconstruction accepts finite coordinates. It clips encoded channels to `[0,1]` and rounds `255*s` to the nearest byte, with ties upward. Negative linear values become byte zero; values above one become 255. Non-finite coordinates are outside the contract. The canonical domain uses the equations in [W3C CSS Color 4](https://www.w3.org/TR/css-color-4/#color-conversion-code); clipping makes a reflected negative transfer extension unnecessary for this byte result.

The image adapters retain alpha in the corresponding RGBA8 source and copy it unchanged during reconstruction. They preserve row padding and hidden RGB, require equal dimensions, and perform no compositing. This follows the [sRGB reconstruction contract](srgb.md).

Independent vectors include byte 10 decoding to approximately 0.0030352698, byte 11 to 0.0033465358, and linear 0.18 encoding to byte 118. Tests cover both sides of each transfer threshold.
