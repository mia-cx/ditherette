---
'ditherette-web': patch
---

Use exact GPU channel tables for eligible image effects in Source previews and output processing. Preserve alpha, fall back to CPU for unsupported chains or GPU failures, and discard results superseded by newer edits.
