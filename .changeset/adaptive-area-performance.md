---
'ditherette': patch
---

Reduce smooth-area analysis work while preserving its exact mask, coverage decisions and rendered output. Skip already-minimal reconstruction distances and avoid repeated square roots in neighbourhood maxima. Memory limits, cancellation checks and legacy rendering stay unchanged.
