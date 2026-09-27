# Private effects boundary

`privateApplyEffects(input, sourceWidth, sourceHeight, effects, palette, space, resultSink)`
applies an ordered chain and writes durable RGBA8 to the caught result sink.

`effects` is the wrapper's normalized JSON array. `palette` is either undefined or the
compact palette codes used by `privateQuantize`. `space` is a working-space tag, or -1 when
the caller supplied none. The wrapper validates first with indexed paths such as
`effects.2.gamma`; Rust rejections here report path 39 (`effects`), 40 (`context.palette`),
or 41 (`context.space`).

The processor snapshots the source with the same identity rules as `perturb`. The result
is retained under the source identity plus the enabled effects, so a repeated call returns
the cached image. Progress reports `prepare`, `effects`, then `complete`.
