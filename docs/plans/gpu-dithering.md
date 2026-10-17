# GPU dithering and Adaptive placement

## Outcome and boundary

The next useful GPU milestone runs ordinary ordered/noise dithering and the complete
Adaptive selector on GPU, returning palette indices. The CPU follow-up remains useful:
the first optimization roughly halved diagnostic latency, but selection still dominates.
This document records the implementation route; it does not authorize deployment or
change the frozen CPU output, cache policy or memory limits.

Use the existing stages: [matching](https://github.com/mia-cx/ditherette/issues/255),
[ordered/noise plus Adaptive](https://github.com/mia-cx/ditherette/issues/256),
[Yliluoma](https://github.com/mia-cx/ditherette/issues/257), and
[diffusion](https://github.com/mia-cx/ditherette/issues/258), under the
[integration umbrella](https://github.com/mia-cx/ditherette/issues/366).

GPU floating-point differences and GPU-produced exports are already accepted in
[the port decision](https://github.com/mia-cx/ditherette/issues/249).
Measure coverage and palette-index disagreements separately from largest RGBA byte
error. Every output pixel still comes from the exact palette. Preserve the accepted
mask geometry, field phase, strength and diffusion recurrence.

## 1. Establish one asynchronous processing job

Reuse the current wgpu device, detection, pipeline ownership and known-answer checks.
Add an internal multi-pass job beside `prod/gpu/stage.rs`: its current `Stage` contract
reads RGBA back after each pass and cannot represent this pipeline efficiently.
Keep intermediate buffers resident until the final indexed readback.

Upload the actual prepared pixels after effects and resize. The existing effects
lookup processes unique RGB values, so it does not leave the full prepared image
resident on GPU. Preserve alpha/matte handling, resize anchors and global coordinates.
Retain at most one admitted active source generation; source/effects/resize/alpha
changes invalidate it. Reuse across settings requires an explicit bounded ownership
policy and review before implementation.

Expose asynchronous GPU processing without changing the existing synchronous CPU API.
Keep one active job per facade, reject stale generations, and check cancellation between
bounded dispatch batches. On CPU fallback, retire the owning GPU artifact and callbacks
before scalar allocation, following the existing effects facade's lifetime contract.

Done when a known indexed image round-trips through the real package/browser worker,
and no-adapter, device-loss, cancellation and memory rejection recover without stale
results, leaked callbacks or overlapping GPU/scalar ownership.

## 2. Port matching and independent fields

Port all 15 matching policies, palette-order tie breaks and transparent-entry rules.
Then port Bayer, rectangular ordered fields, blue noise and seeded random fields.
Each invocation can compute an independent pixel; retain the selected working space,
feedback choice, field coordinates and random seed.

Return packed indices through a mapped buffer. Assign each packed output word to one
writer, or pack in a final pass: separate invocations must not write different bytes
of the same word. CPU reconstruction and PNG encoding consume those indices and the
palette; computation never reads pixels back through a canvas.

Done when every claimed mode/space runs through the actual worker, exports only palette
colours, and reports differences against the CPU on fixtures and full Celeste at 1×.
This foundation alone does not complete the Adaptive milestone.

## 3. Port the entire Adaptive mask

Follow `prod/dither/smooth_area_v1.rs` and its reference in this order:

| CPU phase                                                             | GPU route and completion criterion                                                                                                                                       |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Oklab conversion and boundary witnesses                               | Independent pixels; immutable coordinates plus separate flags avoid races from the CPU's lightness-sign encoding.                                                        |
| Symmetric coarse means, fine/coarse contrast, square maxima and seeds | Separable tiled windows; preserve symmetric clipping and confirm the compact-step/broad-ramp counterexample.                                                             |
| Geometric edge shadow and regional feathering                         | Ordered or bounded parallel distance passes; compare the complete resulting planes.                                                                                      |
| Eight-connected seed retention                                        | Tiled integer component labels, merging and counts; prove completion on long thin paths and cross-tile components. A one-pixel-per-dispatch label crawl is insufficient. |
| Source-aware reconstruction                                           | Preserve all four directional chamfer sweeps, neighbour order, colour barriers and diagonal checks using dependency-aware wavefronts or skewed tiles.                    |
| Final density and coverage                                            | Preserve smoothstep behavior within documented GPU precision and the independent blue-noise coverage decision, including its fixed offset.                               |

Prototype component retention and reconstruction first: they determine whether the
whole selector will be fast. Rectangular tiles can have cyclic dependencies where a
previous row's upper-right pixel is needed. Prove the tile dependency graph before
coding a scheduler. Use dispatch boundaries between dependent workgroups; never wait
inside a workgroup for an unscheduled workgroup to progress.

Keep the four accepted sweeps. Replacing them with jump flooding, a fully converged
geodesic solver or ordinary blur changes the mask, beyond accepted arithmetic rounding.
WGSL concrete floating types are f32/f16, its core atomics use integers, and collective
barriers synchronize within a workgroup. These constraints are documented in the
[WGSL specification](https://www.w3.org/TR/WGSL/#types) and
[synchronization rules](https://www.w3.org/TR/WGSL/#synchronization-builtin-functions).
CPU f64 means/smoothstep therefore need a measured GPU precision implementation.

Initially admit the editor's 4–100 px sample range. Wider valid package requests can
fall back to CPU. This editor value is square side length; its coarse radius is
floor(sampleSize / 2). Preserve the package's existing 4–32768 API range.

Done when conversion through coverage stays on GPU, the accepted lineart/texture/ramp
fixtures retain their appearance, and final ordered/noise output needs only indexed
readback. Report component passes, reconstruction dispatches and end-to-end latency;
a shader-only timer does not establish an interactive improvement.

## 4. Keep diffusion texture while removing Adaptive overhead

Feed the GPU's final coverage decisions to ordinary CPU diffusion as an interim path.
A bit-packed mask avoids reading a full f32 plane back. This needs an internal
validated mask input and complete accounting, not a new public recipe variant.
Diffusion still runs its normal continuous recurrence and masks only visible output.

Serpentine Sierra has a serial dependency through successive pixels and the turn into
the next row. A simple diagonal wavefront cannot preserve that scan. Keep CPU Sierra
available while a separate feasibility experiment measures faithful GPU scheduling.
Turning off serpentine, restarting error at tile edges or unordered floating-point
accumulation would change its texture and requires a separate product decision.

Non-serpentine kernels permit causal wavefronts: for the current taps, Sierra can use
logical time x + 3y and Floyd–Steinberg x + 2y. Coarsen the schedule to avoid a dispatch
per diagonal. Gather predecessor contributions in the defined order. Yliluoma's
per-pixel mixture search is a separate parallel stage with bounded candidate storage.

Done when each supported diffusion kernel preserves its scan and feedback settings,
with measured output differences and a genuine complete-call benefit. Keep unsupported
kernels on CPU; declare their backend explicitly in evidence.

## 5. Admit memory and qualify the complete call

The in-progress implementation is `prod/gpu/area.rs`. Its skew coordinates
`u=x+y` turn each sweep's predecessors into `(-1,0), (-2,-1), (-1,-1), (0,-1)`.
Dispatches advance tile diagonals; barriers advance diagonals within a tile.
Atomic union/find retains complete eight-connected regions without iterative
one-pixel label propagation. These schedules pass native cross-tile/diagonal/thin-path
checks against the unchanged CPU algorithms.

| Lifetime | Owned storage |
| --- | --- |
| Source capture through upload | Host snapshot, Wasm copy, GPU packed RGBA |
| Complete analysis | Packed f32 triplet cells and one triplet work plane |
| Region retention | Work plane reused for integer parents and component counts |
| Reconstruction | Cells plus source; work plane becomes available for output |
| Final readback | Work plane reused for packed coverage or indices; mapped staging and independently owned result |
| Dispatch setup | Immutable aligned uniforms on host and GPU, dispatch metadata and bounded palette/settings |

`AreaMemory` and `IndexedMemory` admit these lifetimes before workspace creation.
Native full Celeste coverage at 20/64/100 px differs by 1/1/0 pixels respectively;
180 field/matching/alpha fixture cases have zero differing indices. This is preliminary
native validation, not browser qualification or completion of the integration milestone.

Create a resource-lifetime table before allocation: source snapshots, Wasm wire copies,
GPU source, coordinate/contrast/distance planes, component labels/counts, staging and
readback. Reuse dead workspaces, honor actual device binding/texture/dispatch limits,
and test exact budget boundaries. A full Celeste vec4<f32> plane alone is 173,388,800
bytes, much larger than the effects lookup; do not copy its budget assumptions.
Keep resource rejection and scalar fallback within the caller's existing limit.

Use full 2600×4168 Celeste at 1× with all 64 palette entries: sample sizes 20/40/60/100,
6%/12% contrast, Bayer16 and Sierra at 30%/100%. Include hard lineart, texture, stars,
diagonals, compact/broad ramps, narrow/padded images, transparent pixels and repeated
source/settings changes. Record density error, coverage disagreements, index changes
and largest RGBA error independently. Keep the private image outside git and uploads.

Verify the installed package in real browser workers and native backends. Record the
physical adapter and execution backend. Separate cold creation, upload, dispatch,
readback, CPU reconstruction and warm setting changes. Compare against the optimized
CPU artifact using the existing exclusive lease and per-pair host evidence. Busy-host
observations remain diagnostic; no speedup claim comes from profiling samples alone.
