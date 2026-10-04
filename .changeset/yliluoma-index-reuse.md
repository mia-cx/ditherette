---
'ditherette': patch
---

Process new Yliluoma frames faster when the palette, matching, and matrix size stay the same. The processor keeps the mixture index between calls, so an 8×8 frame takes about 26% less time and a 16×16 frame about 44% less. Output is unchanged.
