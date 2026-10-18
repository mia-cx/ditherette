---
'ditherette': minor
---

Select CPU or GPU processing through `createDitherette({ engine: 'cpu' | 'gpu', tiling: true | false })`. Both engines provide the same processing operations. GPU calls are asynchronous and share one device across effects, resizing and dithering; CPU-only recipe analysis follows the explicit fallback policy. Existing CPU calls and compatibility GPU factories retain their defaults.
