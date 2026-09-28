---
'ditherette': patch
---

Apply hue and saturation about 5× faster on large photos with identical output. Pointwise effect chains now cache final bytes per colour in a table sized to the image, and no longer allocate a full-image float carrier.
