# Global-index random reference

Each pixel owns exactly one Mulberry32 draw. Global pixel index is `i=y*full_image_width+x`, regardless of row bands.
The draw index is `(i+1) mod 2^32`. Initialize its state as `seed + draw_index*0x6d2b79f5`, modulo 2^32.
Apply the website's [Mulberry32 mixer](../../../../../src/lib/processing/quantize-row.worker.ts):

```text
v = (v XOR (v >> 15)) * (v OR 1)
v = v XOR (v + (v XOR (v >> 7)) * (v OR 61))
draw = v XOR (v >> 14)
```

Every addition and multiplication wraps to unsigned 32 bits; shifts are logical.
Convert draw to f64, divide by 4294967296, subtract 0.5, then round once to f32.
The exact threshold lies in `[-0.5,0.5)`; final f32 rounding can reach 0.5.
One scalar threshold offsets all three working coordinates. There are no per-channel draws.
Alpha, zero strength, and a zero placement mask never remove or shift a pixel's draw assignment.
There is no mutable random state shared between calls, tiles, or workers.

`random_u32_at` exposes the exact draw for sequence fixtures. `random_noise_at` supplies the centered field.
The legacy `Mulberry32` stream and indexed adapters remain available. Its `next_f32` rounds before centering, unlike the new field.
Both APIs use the same integer mixer and assign the same integer draw at each global index.
