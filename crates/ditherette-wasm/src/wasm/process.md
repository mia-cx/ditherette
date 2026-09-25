# Private process boundary

`privateProcess(input, sourceWidth, sourceHeight, palette, version,
outputWidth, outputHeight, algorithm, anchor, support, matching,
alphaMode, alphaThreshold, matte, family, field, parameter, space,
strength, placement, radius, threshold, softness, resultSink)` uses borrowed
externrefs, f64 controls, and a caught void indexed-result sink.

Resize, palette, matching, alpha, and dither encodings are unchanged from
`privateResize`, `privateQuantize`, and `privateDitherAndQuantize`. Version must
equal one before narrowing. Private path 37 appends `recipe.version`.

The adapter takes the instance before palette getters or caught copy helpers.
Every expected error restores readiness. The processor preflights simultaneous
resize preparation, matching/diffusion preparation, source, resized RGBA8,
optional perturbed RGBA8, indices, and boundary records before input copying.
Only final indexed completion publishes JS output. Progress callbacks and the
retained preparation store follow the processor rules in `processor.md`.

The public wrapper maps settings failures into `recipe.output`, `recipe.alpha`,
`recipe.match`, and `recipe.dither`. Source, palette, lifecycle, memory, and
final result-copy failures keep their existing paths. In particular, a failed
final result copy remains `output`, not `recipe.output`.

## Recipe v2

`privateProcessEffects` takes the same arguments plus `effects`, a JSON string, before `resultSink`.
Version must equal two. The wrapper has already validated the effects with indexed paths.
Rust decodes and validates them again; a rejection reports private path 39, `effects`.
Effects read the request palette and the matching working space. Paths 40 and 41 report a missing palette colour or space.
The processor applies effects while `process` snapshots its source, so progress reports `effects` after `prepare`.
A chain with no enabled step runs exactly the v1 `process` path.
