# S36 and S37 indexed row-band reuse

Read each slice and the current execution contract. Start from validated S34, not the earlier scalar package.
S36 owns quantization and separable fields. S37 owns Yliluoma. Diffusion stays scalar.
Each owner uses an isolated worktree. Root reconciles shared pipeline registration at the integration join.

## Existing semantic paths

`PreparedQuantizer` already owns the ordered palette, matcher, and packed converter.
Its quantize loop allocates nothing and writes one index per source pixel. Share immutable preparation across workers.
The public pipeline converts coordinates inline; it does not require a retained full-image color plane.
Preserve alpha preparation, original palette indices, tie order, metadata, and warnings.

The existing perturb row function receives a full-source view and absolute row coordinates.
Its field callback uses `y * full_width + x`, once per written pixel, even when the effect is suppressed.
Adaptive placement reads the unmodified full source, including neighbors outside an output band.
The current output parameter is a complete-image view. Introduce a safe disjoint-band adapter without sharing mutable whole-image views.
Preserve the RGBA8 reconstruction boundary before separable quantization.

The Yliluoma implementation is spelled `prod/dither/yiluoma` in existing source paths.
Its request loop shares prepared palette matching, performs literal ordered pair/level search, and uses global Bayer coordinates.
Adaptive placement uses original source bytes; alpha-adjusted RGB supplies matching coordinates.
Keep these inputs distinct. Preserve strict-less-than tie handling and componentwise mixture arithmetic.
The prior S29 converter candidate remains unselected. Threading does not authorize silently selecting that separate optimization.

## Integration and evidence

`pipeline/indexed.rs` is the shared complete-call path for direct, separable, diffusion, and Yliluoma stages.
Keep its content identities independent of worker count and band size. Cache hits must retain existing behavior.
Callbacks stay on the calling thread. Worker closures must not borrow JavaScript callbacks or private instance state.
Preserve S33 callback failure, durable output, and success-only publication around all scheduled work.
Count worker scratch, plans, temporary converters, and any duplicated preparation before execution.

Reuse frozen tiling geometry and existing production worker-budget helpers. Add only missing execution adapters.
Extend benchmark subjects before optimizing. Native scoped-thread timings do not substitute for pooled public Wasm calls.
Use small, bounded Yliluoma cases because work grows with palette pairs and ordered levels.
Check worker counts, band boundaries, random identities, adaptive seams, and exact scalar output before measurement.
Only measured complete-call wins select a threaded path. A documented scalar winner completes S37.
No frozen-spec edits, public backend controls, or routine PR review belong to either slice.
