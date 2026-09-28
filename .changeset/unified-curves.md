---
'ditherette': minor
---

Replace `curves`, `model-curves`, and `channel-curve` with one ordered `curves` effect. Each curve now declares its remap or adjustment kind, input and output channels, and control points. Remaps use an open spline, while hue-input adjustments wrap across a closed seam.
