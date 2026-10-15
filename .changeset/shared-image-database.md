---
'ditherette-web': patch
---

Reuse the saved-image database connection across reads and writes, and recover after connection closure or a failed open.
