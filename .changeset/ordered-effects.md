---
'ditherette': minor
---

Add ordered colour effects. `applyEffects` runs an effect chain and returns full-colour RGBA8. Recipe version 2 adds `effects` to `process`; they run on the source before resize, dithering, and quantization. The first built-in effect is `levels`. Version 1 recipes are unchanged.
