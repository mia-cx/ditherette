---
'ditherette': patch
---

Load the WebGPU glue with a static lazy import, so bundlers include it in their output. Single-file bundles such as userscripts and browser extensions now run on the GPU instead of falling back to the CPU, without a build-time patch.
