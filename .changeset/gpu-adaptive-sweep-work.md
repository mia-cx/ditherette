---
'ditherette': patch
---

Trim GPU Adaptive sweep work without changing coverage. Sweeps launch only tiles that touch the image, and each tile prepares its colour gates and neighbour distances before the serial steps.
