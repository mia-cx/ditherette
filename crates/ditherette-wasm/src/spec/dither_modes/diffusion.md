# Extended error-diffusion reference

`diffusion::diffuse(QuantizeRequest, DiffusionPolicy)` runs the [v1 diffusion recipe](../dither/error_diffusion.md) with any `Kernel`.
Feedback, scan order, alpha sinks, placement, f64 scatter arithmetic, and overflow errors are exactly the v1 rules.
Only the tap table changes. For `floyd-steinberg`, `sierra`, `sierra-lite`, and `atkinson` the result equals v1 `diffuse`.

## Tap sets

Each triple is `(dx, dy, numerator)`. Divide by the listed denominator; the weight is that f32 quotient.
Taps scatter in the listed order. Serpentine rows negate `dx`, as in v1.
Out-of-image taps are dropped, not renormalised.

| Tag                   | Denominator | Ordered taps                                                                                                     | Source                                                     |
| --------------------- | ----------- | ---------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| `jarvis-judice-ninke` | 48          | `(1,0,7), (2,0,5), (-2,1,3), (-1,1,5), (0,1,7), (1,1,5), (2,1,3), (-2,2,1), (-1,2,3), (0,2,5), (1,2,3), (2,2,1)` | [Helland][helland], [libcaca study][caca]                  |
| `stucki`              | 42          | `(1,0,8), (2,0,4), (-2,1,2), (-1,1,4), (0,1,8), (1,1,4), (2,1,2), (-2,2,1), (-1,2,2), (0,2,4), (1,2,2), (2,2,1)` | [Helland][helland], [libcaca study][caca]                  |
| `burkes`              | 32          | `(1,0,8), (2,0,4), (-2,1,2), (-1,1,4), (0,1,8), (1,1,4), (2,1,2)`                                                | [Helland][helland], [libcaca study][caca]                  |
| `two-row-sierra`      | 16          | `(1,0,4), (2,0,3), (-2,1,1), (-1,1,2), (0,1,3), (1,1,2), (2,1,1)`                                                | [Helland][helland], [libcaca study][caca]                  |
| `fan`                 | 16          | `(1,0,7), (-2,1,1), (-1,1,3), (0,1,5)`                                                                           | [libcaca study][caca]                                      |
| `shiau-fan`           | 8           | `(1,0,4), (-2,1,1), (-1,1,1), (0,1,2)`                                                                           | [libcaca study][caca], [US 5,353,127][patent]              |
| `shiau-fan-2`         | 16          | `(1,0,8), (-3,1,1), (-2,1,1), (-1,1,2), (0,1,4)`                                                                 | [libcaca study][caca], [US 5,353,127][patent]              |
| `simple-2d`           | 2           | `(1,0,1), (0,1,1)`                                                                                               | [ditherit][ditherit-simple], [makew0rld/dither][dither-go] |

The v1 kernels keep their [v1 tables](../dither/error_diffusion.md#tap-sets).
Every listed kernel distributes total weight 1. Shiau-Fan 2 reaches three pixels left, so every tap still fits three rows.
[ditherit][ditherit] uses the same numbers under the names `JarvisJudiceNinke`, `Stucki`, `Burkes`, `Sierra2`, `Fan`, `ShiauFan`, and `ShiauFan2`.
Its `Simple2D` scans raster only. Here serpentine applies to it as to every kernel; its `(0,1)` tap is unchanged by mirroring.

[helland]: https://tannerhelland.com/2012/12/28/dithering-eleven-algorithms-source-code.html
[caca]: http://caca.zoy.org/study/part3.html
[patent]: https://patents.google.com/patent/US5353127A
[ditherit]: https://github.com/alexharris/ditherit/blob/8a8e053fa149160dad32526bfab89a38bba6e6d4/app/utils/dithering.ts
[ditherit-simple]: https://github.com/alexharris/ditherit/commit/901a64bb05dbecdc4d5e50a21b8211416940895a
[dither-go]: https://github.com/makew0rld/dither/blob/master/error_diffusers.go
