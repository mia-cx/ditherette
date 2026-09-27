---
'ditherette': minor
---

Add palette-aware recolouring. `analyzeRecolour` derives an editable recipe from an image and a palette in the chosen working space, and the `recolour` effect applies it at any strength, or analyses automatically inside a chain. Analyses are cached, so later edits never re-analyse.
