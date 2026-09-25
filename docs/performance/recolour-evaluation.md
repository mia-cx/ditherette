# Recolouring: treated versus untreated

Automatic recolouring (#201) fits an image to what a palette can show before quantization.
This compares the same images quantized with and without a leading `recolour` step, and records what analysis and application cost.
Recolouring does not help every image. Where it hurts, the table says so.

## Method

`cargo run --release --example recolour_evaluation [sheet-directory]` in `crates/ditherette-bench` reproduces the table.
With a directory argument it also writes one PNG sheet per case: source, untreated, treated.

Each of the four benchmark fixtures is area-resized to 320 px wide with the reference.
The same image then runs through recipe-v2 `process` twice, matching in Oklab: with no effects, and with one automatic `recolour` step at strength 1.
Six palettes cover the range from rich to tiny: Wplace's 31 free colours, PICO-8's 16, the four-colour Game Boy greens, four warm colours, five cool colours, and four greys.
Three dither families run for each: none, Bayer 8 (separable, strength 0.5), and Floyd–Steinberg (serpentine, matching feedback).
Values are means over the four fixtures.

Every metric compares the indexed output with the continuous source, in Oklab:

| Metric | Meaning | Better |
| --- | --- | --- |
| Shift ΔE | Mean distance between 5×5-blurred output and source: the colour the eye averages dithering into | lower |
| Detail r | Correlation between blurred output and source lightness gradients | higher |
| Colour loss | Share of clearly coloured source pixels (chroma > 0.04) that come out near-grey (< 0.02) | lower |
| Colours used | Distinct palette entries in the output | higher |
| Texture | Mean difference between output lightness and its blur: visible dithering noise | lower |

## Results

Arrows read untreated → treated.

| Palette | Dither | Shift ΔE | Detail r | Colour loss | Colours used | Texture |
| --- | --- | --- | --- | --- | --- | --- |
| wplace-free | none | 0.0571 → 0.0584 | 0.911 → 0.907 | 23.9% → 19.5% | 21.2 → 22.5 | 0.0346 → 0.0360 |
| wplace-free | bayer-8 | 0.0397 → 0.0436 | 0.855 → 0.884 | 6.4% → 5.0% | 24.0 → 24.5 | 0.0557 → 0.0553 |
| wplace-free | floyd-steinberg | 0.0120 → 0.0228 | 0.885 → 0.862 | 0.4% → 0.2% | 28.5 → 28.8 | 0.0956 → 0.1004 |
| pico-8 | none | 0.0680 → 0.0722 | 0.890 → 0.890 | 13.9% → 10.6% | 13.8 → 14.2 | 0.0348 → 0.0361 |
| pico-8 | bayer-8 | 0.0561 → 0.0606 | 0.863 → 0.865 | 9.7% → 8.2% | 14.8 → 15.0 | 0.0560 → 0.0576 |
| pico-8 | floyd-steinberg | 0.0158 → 0.0313 | 0.816 → 0.796 | 0.2% → 0.1% | 15.5 → 15.8 | 0.1273 → 0.1307 |
| gameboy | none | 0.1405 → 0.1547 | 0.828 → 0.811 | 0.0% → 0.0% | 4.0 → 4.0 | 0.0310 → 0.0309 |
| gameboy | bayer-8 | 0.1365 → 0.1523 | 0.748 → 0.798 | 0.0% → 0.0% | 4.0 → 4.0 | 0.0486 → 0.0510 |
| gameboy | floyd-steinberg | 0.1455 → 0.1528 | 0.402 → 0.616 | 0.0% → 0.0% | 3.8 → 4.0 | 0.0508 → 0.0899 |
| warm-4 | none | 0.1478 → 0.1978 | 0.806 → 0.714 | 0.0% → 0.0% | 4.0 → 4.0 | 0.0363 → 0.0369 |
| warm-4 | bayer-8 | 0.1444 → 0.1942 | 0.810 → 0.707 | 0.0% → 0.0% | 4.0 → 4.0 | 0.0470 → 0.0591 |
| warm-4 | floyd-steinberg | 0.1098 → 0.1654 | 0.467 → 0.543 | 0.0% → 0.0% | 3.5 → 4.0 | 0.1666 → 0.1653 |
| cool-5 | none | 0.1645 → 0.1963 | 0.796 → 0.772 | 0.0% → 0.0% | 5.0 → 5.0 | 0.0196 → 0.0232 |
| cool-5 | bayer-8 | 0.1580 → 0.1941 | 0.684 → 0.733 | 0.0% → 0.0% | 5.0 → 5.0 | 0.0348 → 0.0438 |
| cool-5 | floyd-steinberg | 0.1686 → 0.1895 | 0.349 → 0.602 | 0.0% → 0.0% | 4.0 → 5.0 | 0.0339 → 0.0750 |
| grey-4 | none | 0.1008 → 0.1113 | 0.753 → 0.795 | 100.0% → 100.0% | 3.8 → 4.0 | 0.0367 → 0.0536 |
| grey-4 | bayer-8 | 0.0860 → 0.1020 | 0.666 → 0.787 | 100.0% → 100.0% | 4.0 → 4.0 | 0.0626 → 0.0730 |
| grey-4 | floyd-steinberg | 0.0632 → 0.0871 | 0.804 → 0.855 | 100.0% → 100.0% | 4.0 → 4.0 | 0.1206 → 0.1194 |

## Reading the table

**Small palettes with dithering gain the most.** Floyd–Steinberg detail rises from 0.40 to 0.62 on Game Boy, 0.35 to 0.60 on cool-5, and 0.47 to 0.54 on warm-4.
Untreated, most of the image falls between two palette entries and dithers into flat texture. Treated, tones spread across the entries the palette has, so faces and folds keep their shading.
Every small palette also uses all of its colours after treatment.

**Rich palettes change little.** With Wplace and PICO-8 the treatment mostly leaves tones alone, since the palette already spans them.
It keeps more colour: colour loss without dithering falls from 24% to 20% (Wplace) and 14% to 11% (PICO-8), because muted colours no longer collapse onto greys.
The cost is a small shift, largest with Floyd–Steinberg (0.012 → 0.023 ΔE on Wplace), where diffusion already reproduced the source well.

**Shift always grows.** That is the intent: the treatment moves colours toward what the palette can show, so the result is further from the source and closer to the palette.
It is the reason `strength` exists.

**Where it hurts.** Warm-4 without dithering loses detail (0.81 → 0.71): the treatment lifts shadows to the palette's darkest colour, deep red, and compresses contrast between dark regions.
Grey-4 texture rises without dithering, since tones spread into more steps. Neither is a clear loss visually, but neither is a clear win. Lower `strength`, or edit the `tone` curve.

## Cost

Native x86-64 release, Criterion `crit_effects`, 8-colour palette, Oklab. Analysis and application are measured separately. Full tables are in [`prod/effects/README.md`](../../crates/ditherette-wasm/src/prod/effects/README.md).

| Work | 800×800 | 3462×2309 |
| --- | ---: | ---: |
| Analysis | 17.8 ms | 44.8 ms |
| Application of a known recipe | 9.5 ms | 225.7 ms |

Analysis reads at most 2¹⁸ samples, so it grows slowly with image size. Application runs once per distinct colour through a memo, so flat illustrations are cheaper than photos.

Public package, scalar Wasm, Node 24, 3462×2309 synthetic source, median of five, milliseconds:

| Call | Cold | Warm |
| --- | ---: | ---: |
| `analyzeRecolour` | 163.5 | 63.5 |
| `applyEffects` with a recipe | 373.6 | 20.0 |
| `applyEffects`, automatic | 475.4 | 19.4 |
| `process` v2, automatic, to 480×320 | 515.0 | 4.9 |

Cold calls use a fresh processor. A warm `analyzeRecolour` hits the analysis cache; warm `applyEffects` returns its retained result; warm `process` skips the effects entirely because the source and chain repeat.
Adding an effect after the recolour step reuses its cached analysis: that call took 543 ms, all of it application and processing.
