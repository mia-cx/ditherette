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
Only final indexed completion publishes JS output. No callback or cache work runs.

The public wrapper maps settings failures into `recipe.output`, `recipe.alpha`,
`recipe.match`, and `recipe.dither`. Source, palette, lifecycle, memory, and
final result-copy failures keep their existing paths. In particular, a failed
final result copy remains `output`, not `recipe.output`.
