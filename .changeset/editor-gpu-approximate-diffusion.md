---
'ditherette-web': patch
---

Error diffusion dithers on the GPU when the browser has WebGPU. The result looks the same as before, though individual pixels differ, and serpentine Sierra on a 10-megapixel image no longer waits on the CPU.
