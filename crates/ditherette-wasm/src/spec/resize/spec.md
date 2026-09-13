# Resize reference

## Complete operation

`spec::resize::resize` validates a version-one `ResizeRequest` and returns owned RGBA8.
It dispatches directly to the naive kernels below. It does not call production code.
The operation samples channels independently; palette-dependent alpha preparation happens after resize.

Generic kernels accept validated `ImageView<F>` and `ImageViewMut<F>`.
Their logical rows may have padding. `ResizeSample` defines accumulation and storage rounding for byte and float channels.
The public package accepts cropped, packed RGBA8 only.

## Executable oracle map

| Recipe or helper | Reference |
| --- | --- |
| Complete request | `resize::resize` |
| Nearest | `scalar::nearest::resize_nearest_into` |
| Area | `scalar::area::resize_area_into` |
| Scale-aware triangle | `scalar::bilinear::resize_bilinear_into` |
| Catmull-Rom cubic | `scalar::bicubic::resize_bicubic_into` |
| Lanczos2/3 | `scalar::lanczos::resize_lanczos2_into`, `resize_lanczos3_into` |
| Positive integer Lanczos radius | `scalar::lanczos::resize_lanczos_into` |
| Reconstruction kernel | `scalar::convolution::resize_convolution_into` and `ReconstructionKernel` |
| Trilinear mip policy | `scalar::trilinear::resize_trilinear_into` |
| Integer coordinate mapping | `common::alignment::map_axis_coordinate` |
| Continuous coordinate mapping | `common::coordinates::map_axis_position` |
| Storage conversion | `common::sample::ResizeSample` |

Cubic and Lanczos expose fixed and scale-aware support. Bilinear always widens its triangle during minification.
All coordinate-based recipes accept the nine axis-anchor combinations. Area integrates source footprints and has no anchor.
Center remains the default of the generic anchor type; typed requests specify their recipe explicitly.

## Mathematical rules

Output positions use complete-image coordinates. Edge samples clamp to the source image, never to a tile.
Area accumulates exact rectangular overlap. Reconstruction filters evaluate each support weight directly and normalize their sum.
Byte output clamps to 0..255 and rounds halfway upward; float output retains its fractional value in f32 storage.

Trilinear builds separate naive area mip chains for its selected levels.
Each dimension halves with ceiling division. LOD is the base-two logarithm of the larger source/output axis ratio.
Magnification delegates to the same bilinear reference. Minification resizes selected mip levels, then blends their stored results.
Storage rounding therefore happens at each area mip, each bilinear output, and the final blend.
Mip construction copies logical rows, excluding source padding.

## Verification

The existing nearest/filter fixtures and `tests/spec_resize_contract.rs` cover every recipe through the complete request.
They check all nine anchors, nonconstant identity, one-pixel magnification, symmetric averages, and malformed inputs.
Independent calculations cover cubic support widening, fractional LOD, odd mip dimensions, staged byte rounding, anisotropic LOD, and float samples.
A padded input/output fixture protects logical-row behavior. Lanczos kernel fixtures check known half-angle sine values and finite support.

## Production obligations

Complete and freeze the reference first. Copy the implementation into mirrored production modules and verify the copied baseline.
Optimization then changes production only. Exact candidates retain these bytes and coordinate rules.
A differing candidate needs Mia's visual acceptance; keep the exact implementation while that acceptance is pending.
Production row and tile adapters compare against the corresponding rows of the complete-image oracle.

This reference does not implement caching, contribution tables, SIMD, threading, or source cropping.
