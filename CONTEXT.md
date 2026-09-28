# Ditherette

Ditherette turns images into palette-limited pixel art: it prepares an image with effects, resizes it, and matches every pixel to a palette colour, optionally through dithering.

## Language

**Curve**:
A smooth function from one or two input channels of a pixel to one output. Two-input curves are still curves; the input count is a setting.
_Avoid_: Surface, mapping, map

**Remap**:
A curve that sets a channel from that same channel. Neutral is the diagonal.

**Adjustment**:
A curve that shifts, scales, or offsets its output channel by an amount read from its inputs. Neutral is a flat line through the middle.
_Avoid_: Correction, offset curve

**Palette fit**:
An effect that analyses the image and the palette together and writes curves that fit the image to what the palette can show, before any matching.
_Avoid_: Recolour (the package's effect name), recolouring

**Mask**:
Curves on an effect step whose output is that step's strength per pixel, such as "only the shadows" or "only the reds".
_Avoid_: Range mask, amounts

**Range input**:
The channel a mask curve reads, from any colour model: lightness, hue, chroma, or an opponent axis such as Oklab a.
