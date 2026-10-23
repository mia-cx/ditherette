---
'ditherette': minor
---

Add `diffusionBackend: 'gpu-approximate'`: error diffusion runs in parallel on the GPU and looks like the original scan, with different exact indices. Bands of rows carry their exact errors downward while row segments diffuse side by side, so serpentine no longer runs one pixel at a time. `gpu` keeps the original scan order and remains the GPU engine's default.
