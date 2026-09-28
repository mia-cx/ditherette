# Two-input curves in existing colour tools

Ditherette's curve effect (`crates/ditherette-wasm/src/prod/effects/curves.rs`, spec at `crates/ditherette-wasm/src/spec/effects/curves.md`) takes one input channel and drives one output through a monotone cubic Hermite spline. A planned extension takes **two** input channels, for example hue and lightness, driving one output, for example saturation. The curves spec's non-goals list "hue-versus-saturation curves" for this reason: it needs its own design.

This surveys how established colour tools handle curves with two inputs, to give issue #293 real prior art instead of a blank page. The central finding: almost none of them ship a true two-input curve. Most represent "hue vs saturation" style tools as **one shared input axis (usually hue) driving several independent 1D output curves**, each curve controlling a different output quantity (saturation, luminance, hue rotation, per-channel gain). That is a separable design, not a joint function of two inputs. The only tools that touch a genuine second axis do it through masking (Adobe's range masks) or leave the exact mechanism undocumented (Baselight).

## DaVinci Resolve: HSL Curves

Source: DaVinci Resolve reference manual, chapter 26 "Curves" (official Blackmagic Design PDF, `documents.blackmagicdesign.com/UserManuals/DaVinci-Resolve-20-Fusion-Visual-Effects.pdf`, pages 654-660).

Resolve ships five HSL curves: Hue vs Hue, Hue vs Sat, Hue vs Lum, Lum vs Sat, Sat vs Sat.

1. **Representation.** Each curve is a 1D curve with 2 or more `[x, y]` control points, same as Ditherette's tone curve. The x-axis is always one single quantity: hue for the first three curves, luma for Lum vs Sat, saturation for Sat vs Sat. There is no curve keyed on two axes at once, and the manual is explicit that "the range of hue that you isolate with control points is always relative to the RGB input connected to that node": hue, sat, and luma are computed once per pixel and shared across whichever HSL curves are open on that node.
2. **Interpolation.** The manual does not name the spline algorithm for the default control-point mode. A toggle ("Enable Bezier") switches a control point to exposed Bezier handles for manual tangent control instead of Resolve's default automatic curve. No monotonicity guarantee is documented for either mode.
3. **Hue wrapping.** Circular. Quoting the manual: "the range of hues cycle smoothly from the left to right edge, changes that affect the curve near the left boundary of these curves loop smoothly around to the right boundary, and vice versa, such that the left and right sides of the curve always move together."
4. **Grey/low-saturation handling.** Not documented. The manual does not mention any saturation threshold or grey-pixel special case for the hue-keyed curves.
5. **Editing gestures.** Curves start flat (no adjustment) rather than diagonal (identity), unlike the per-channel Custom curves. Three ways to place points: click directly on the curve; click one of six preset colour swatches (red/yellow/green/cyan/blue/magenta), which drops three points (two bounding, one centred) at that hue; or click-drag over pixels in the viewer, which samples the hue/tonality range under the cursor and places three points to match. Points can be dragged with the mouse or with a dedicated trackball control on DaVinci's colour control panel.

## DaVinci Resolve: HSL Qualifier

Source: DaVinci Resolve 18 manual, "Basic Qualification Using the HSL Qualifier" (`steakunderwater.com`, an HTML mirror of Blackmagic's own manual text).

The HSL Qualifier is not a curve. It is a keyer: three independent range selectors (hue, saturation, luma), each with low/high bounds and soft edges, that combine into one selection matte. Sampling a pixel (or dragging over a region) in the viewer sets all three ranges at once from the sampled pixels. The manual frames the Curves-vs-Qualifier choice directly: curves give smoother, gradual falloff with no hard selection boundary; qualifiers give a sharper, more explicit boundary but can show matte edge artefacts. Worth noting because it is the second and more common way these tools solve "affect only these hues": a selection mask feeding a separate correction, not a curve at all.

## Nuke: HueCorrect

Sources: Foundry's official Nuke reference (`learn.foundry.com/nuke/content/reference_guide/color_nodes/huecorrect.html`), corroborated by the open-source Natron/OpenFX documentation for the equivalent OFX HueCorrect plugin (`natron.readthedocs.io/en/master/plugins/net.sf.openfx.HueCorrect.html`), which documents the same parameter set in more depth. Foundry's own page names the curves but is sparse on axis mechanics; Natron's OFX docs fill the gap since it implements the same node contract.

1. **Representation.** One parametric curve widget with eight tabs, all keyed on hue: `sat` (saturation gain), `lum` (luminance gain), `red`/`green`/`blue` (direct per-channel gain), `r_sup`/`g_sup`/`b_sup` (per-channel suppression, used for green/blue-screen spill). Every tab is a separate 1D curve over the same hue axis, evaluated and applied together per pixel. No tab combines two input axes.
2. **Interpolation.** Neither source names the exact spline type.
3. **Hue wrapping.** Implied circular (hue is inherently periodic and the curve widget is described as looping), but neither source states the wrap behaviour as explicitly as DaVinci's manual does.
4. **Grey/low-saturation handling.** Documented in the OFX/Natron reference: a `sat_thrsh` (saturation threshold) parameter. Below that source saturation, the lum/red/green/blue gain curves are not applied; above it, gain is applied progressively. This is the clearest documented example in the survey of a tool explicitly compensating for hue instability near grey.
5. **Editing gestures.** Standard curve widget: click to add a point, drag to reshape. `channels` selects which of RGB the correction touches; `mix` and `mix luminance` blend the corrected result back toward the original.

## Baselight (FilmLight)

FilmLight's Baselight manuals sit behind a customer login (`filmlight.ltd.uk/support/documents/baselight/manuals_bl.php`) and are not publicly readable, so this section is limited to what FilmLight publishes openly: a press release for Baselight 6.0 (`filmlight.ltd.uk/store/press_releases/filmlight-releases-baselight-6-0/`) and trade coverage citing FilmLight staff (`digitalmediaworld.tv/post/baselight-6-0-adds-new-colour-space-al-ml-tools-look-dev-for-colourists`).

Baselight ships a "Hue Angle" tool and a "Curve Grade" tool, both reworked in version 6.0 around a new perceptually-uniform "opponent" colour space, which FilmLight says was built so hue and saturation curves stay stable and artefact-free under large adjustments. Beyond that framing, no public source documents the curve's control-point structure, interpolation, hue wrapping, or grey-pixel handling. This is a real gap, not an inferred "no": the mechanism may well be a true two-input surface, but nothing publicly available confirms or denies it. Anyone with Baselight customer access should check the manual directly before this informs a decision.

## Adobe Premiere Pro: Lumetri Hue Saturation Curves

Source: Adobe's official help page (`helpx.adobe.com/premiere-pro/using/adjust-color-rgb-hsl-curves.html`; direct fetch was blocked by Adobe's bot protection, so this is drawn from that page's indexed content).

Same pattern as DaVinci: five curves, Hue vs Hue, Hue vs Saturation, Hue vs Luma, Luma vs Saturation, Saturation vs Saturation. Each is a 1D curve over one axis (hue, luma, or saturation) driving one output. The eyedropper sampling gesture matches DaVinci's: clicking a colour in the image drops three control points (two bounding, one centre) on the currently active curve, and a "range" control widens or narrows how far the outer two points sit from the sampled centre. No public documentation found for the interpolation algorithm, wrap behaviour, or grey-pixel handling; the UI is closely convergent with DaVinci's HSL curves.

## Adobe Camera Raw / Lightroom: range masks

Source: Adobe's official help page (`helpx.adobe.com/camera-raw/using/masking.html`; also fetched through indexed content, direct fetch blocked).

Camera Raw and Lightroom have no two-input curve at all. Local adjustments (brush, radial, linear gradient) can be refined afterwards with a **Color Range Mask** or a **Luminance Range Mask**, and only one of the two at a time, not both together. Colour range: sample up to five colours with an eyedropper (shift-click to add), then a single "Amount" slider narrows or widens how close a pixel's colour must be to a sample to count, with soft falloff at the edge. Luminance range: two endpoint sliders plus a smoothness slider define a soft-edged brightness band. Both are masks multiplying an existing adjustment's strength, computed independently of any curve; they are the clearest example in this survey of a tool that fakes "restrict this adjustment by a second attribute" without building any kind of curve over that attribute.

## Houdini and Blender: ramp and LUT editors

Source: SideFX's official ramp parameter documentation (`sidefx.com/docs/houdini/network/ramps.html`) and the Blender manual's compositor node reference (`docs.blender.org`, via the manual's own source at `projects.blender.org/blender/blender-manual`).

Neither tool has a true 2D ramp for this purpose. Houdini's ramp parameter (used throughout COPs, VOPs, and shaders) is strictly 1D: a value or colour varies along a single axis, with a choice of six interpolation bases (constant, linear, Catmull-Rom, monotone cubic, Bezier, B-spline, Hermite) and point-and-drag editing, directly comparable to Ditherette's existing tone curve. Blender's ColorRamp node is likewise 1D (0.0 to 1.0 in, colour out).

Blender's closer analogue to Nuke's HueCorrect is its compositor **Hue Correct node** (`docs.blender.org/manual/en/5.2/compositing/types/color/adjust/hue_correct.html`): one curve widget with three graphs (H, S, V), each a 1D curve over hue. Per the manual, "pixels with hue values [at] each point in the horizontal position of the graph will be changed depending on the shape of the curve," which is the same separable, hue-keyed pattern as DaVinci and Nuke. Blender's release notes (cited via the developer documentation search) record that as of Blender 4.2 the node was fixed to evaluate the saturation and value curves at the pixel's *original* hue rather than its hue after the hue curve already shifted it, and to wrap correctly at red rather than showing a seam. That 4.2 fix is itself useful prior art: it is a documented example of a separable multi-curve design getting its evaluation order wrong (evaluating S and V curves against the post-hue-shift value) and having to be corrected, a trap worth avoiding by design rather than by patch.

## Comparison table

| Tool | True 2-input curve? | Representation | Documented spline | Hue wrap | Grey handling | Edit gesture |
| --- | --- | --- | --- | --- | --- | --- |
| DaVinci Resolve HSL Curves | No (separable) | 5x independent 1D curves, each keyed on hue, luma, or sat | Undocumented; optional manual Bezier mode | Circular, documented | Undocumented | Click, 6 colour presets, viewer sampling |
| DaVinci Resolve HSL Qualifier | No (mask, not curve) | 3 range selectors (hue/sat/luma) with soft edges | N/A | N/A | N/A | Viewer sampling, drag range edges |
| Nuke HueCorrect | No (separable) | 8x independent 1D curves, all keyed on hue | Undocumented | Implied circular, not detailed | Documented: `sat_thrsh` gates gain curves below a saturation threshold | Click/drag on curve widget |
| Baselight Hue Angle / Curve Grade | Unknown, not public | Unknown | Unknown | Unknown | Unknown | Unknown |
| Adobe Premiere Lumetri Curves | No (separable) | 5x independent 1D curves, each keyed on hue, luma, or sat | Undocumented | Undocumented | Undocumented | Click, eyedropper + range width slider |
| Adobe Camera Raw range masks | No (mask, not curve) | Colour: up to 5 sampled points + amount threshold. Luminance: 2 endpoint sliders + smoothness | N/A | N/A | N/A | Eyedropper sampling, single amount/smoothness slider |
| Houdini ramp parameter | No | 1D ramp, 6 interpolation bases available | Documented: constant/linear/Catmull-Rom/monotone cubic/Bezier/B-spline/Hermite | N/A | N/A | Click to add point, drag to move |
| Blender ColorRamp | No | 1D ramp, 0-1 domain | Linear or ease/cardinal options in the widget | N/A | N/A | Click to add stop, drag to move |
| Blender Hue Correct | No (separable) | 3x independent 1D curves (H, S, V), keyed on hue | Undocumented | Circular, fixed in 4.2 | Not documented as a threshold; 4.2 fixed evaluation order instead | Curve widget, click/drag |

## Candidate representations for Ditherette

Three ways #293 could shape a two-input curve. Each keeps compiling to a single LUT, the constraint every option below is built to satisfy; none is a recommendation.

**A. Separable curve stack.** One shared input axis (for example hue), and N independent 1D curves over that axis, each producing a different output or blend weight, combined afterward (multiply, add, or pick by output channel). This is what every hue-based tool in this survey actually ships: DaVinci, Nuke, Premiere, and Blender's Hue Correct are all this shape. It reuses Ditherette's existing `Spline` and `Curves` effect almost unchanged, just re-keyed to a circular hue axis and possibly re-run per output. Cost: it cannot express a value that genuinely depends on *both* inputs jointly, for example "boost saturation for green hues, but only in the shadows" needs two separate curves (hue-keyed and lightness-keyed) whose outputs get combined by some rule chosen outside the curve itself, which is an approximation of a joint function, not an exact one.

**B. Two-dimensional control-point grid.** Control points sit on a rectangular (x1, x2) grid; the output surface between them comes from a 2D spline (bicubic, or a monotone 2D scheme built from Ditherette's existing 1D Fritsch-Butland spline applied along each axis in turn). No tool surveyed here ships this for hue curves; it is a step beyond what any of the prior art in this doc actually does, though it is a common shape for 2D/3D LUT authoring tools more generally. Full joint dependence, exactly the case A can't express. Costs: the editing UX has no precedent from this survey to draw on, a full grid is expensive to keep sparse and editable (16 points was already the ceiling for the 1D case), and one axis being circular (hue) while the other is not (lightness or saturation) needs explicit handling at the grid's hue edge.

**C. Scattered control points with distance-based interpolation.** Points placed anywhere in the 2-input space (not constrained to a grid), each carrying a target output; interpolated by distance, for example inverse-distance weighting or radial basis functions. Closest to how users actually pick colours in the surveyed tools: DaVinci's viewer sampling and Adobe's colour-range eyedropper both drop points where the user clicked in image-derived space, not on a fixed axis. Costs: no monotonicity guarantee comparable to Fritsch-Butland exists for scattered 2D interpolation by default, behaviour between sparse points is harder to reason about or bound, and it is a less familiar mental model for anyone who has used curve tools like the ones surveyed here before.

All three need an explicit answer for near-grey pixels, since hue is undefined at zero saturation and unstable near it: HueCorrect's `sat_thrsh` (a saturation floor below which the curve's effect fades out) is the one documented mechanism in this survey and is a reasonable starting point regardless of which representation #293 picks.
