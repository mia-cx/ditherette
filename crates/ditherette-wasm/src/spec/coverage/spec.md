# Coverage reference

## Purpose

Resize images with transparency without leaking the colour hidden under transparent pixels.

## Contents

| File | Role |
| --- | --- |
| [resize.rs](resize.md) | Coverage-weighted `resize`, premultiply, unpremultiply, main-lobe alpha bounds |

## Composition

Recipe version 2 resizes through this domain; recipe version 1 and the standalone v1 `resize` keep sampling channels independently.
Nearest and fully opaque sources produce exactly the v1 bytes, so only filtered images with transparency change.

## Non-goals

Gamma-correct (linear-light) filtering, alpha-aware nearest, and changes to the frozen v1 kernels.
