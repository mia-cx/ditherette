---
'ditherette': patch
---

Load the WebGPU glue with a static lazy import, so bundlers include it in their output. Single-file bundles such as userscripts and browser extensions no longer need a build-time patch to reach the GPU. They still serve the GPU Wasm or pass it as `gpuWasm`.
