---
'ditherette': minor
---

Add `indexColours`, `compileEffects`, and `applyCompiledEffects`: compile an effect chain once per distinct colour and map pixels through it, byte-exact with `applyEffects`, for live previews on the CPU or GPU.
