---
'ditherette': minor
---

Add `isEffect(value)`, which reports whether one effect step would be accepted, without loading Wasm. Hosts use it to vet saved steps before sending them.
