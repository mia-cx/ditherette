---
'ditherette': patch
---

Reuse Adaptive coverage when changing dither algorithms, strength, palettes or matching spaces. Changed source pixels, image preparation and Adaptive controls invalidate it. Retained coverage uses one bit per pixel within the existing memory and cache limits.
