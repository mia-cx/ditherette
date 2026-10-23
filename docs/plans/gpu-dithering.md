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

| Lifetime                      | Owned storage                                                                                   |
| ----------------------------- | ----------------------------------------------------------------------------------------------- |
| Source capture through upload | Host snapshot, Wasm copy, GPU packed RGBA                                                       |
| Complete analysis             | Immutable Oklab triplets, packed f32 triplet cells and one triplet work plane                   |
| Region retention              | Work plane reused for integer parents and component counts                                      |
| Reconstruction                | Cells plus immutable Oklab coordinates; work plane becomes available for output                 |
| Final readback                | Work plane reused for packed coverage or indices; mapped staging and independently owned result |
| Dispatch setup                | Immutable aligned uniforms on host and GPU, dispatch metadata and bounded palette/settings      |

`AreaMemory` and `IndexedMemory` admit these lifetimes before workspace creation.
The coordinate plane adds twelve requested bytes per pixel to cold analysis,
then dies with the job. It replaces repeated source conversion during contrast
and the four reconstruction sweeps; retained coverage keeps its existing bound.
Native full Celeste coverage at 20/64/100 px differs by 1/1/0 pixels respectively.
Real Chromium workers pass 363 field/matching/alpha/diffusion cases with no index
differences. The 32 full-size Celeste cases compare 346,777,600 indices, with 406
disagreements (four in Sierra results) and maximum reconstructed RGBA byte difference 81. Every output still uses exact palette entries. Three benchmark photos also run
matching, Adaptive Bayer and hybrid Sierra; their per-photo differences and maximum
byte errors are retained separately. Physical qualification currently covers NVIDIA
Ampere only, without a qualified latency claim.

The asynchronous package facade reuses one source snapshot and word-packed coverage,
bounded by the existing 256 MiB / quarter-budget rule. Every source byte is verified
before reuse. A cold retained analysis additionally reads its packed bits; warm fields
upload those bits instead of allocating float planes, and warm diffusion consumes them
directly. Pending bits publish only after successful result copy and completion callbacks.
The full 1× browser matrix exercises eight analyses and 24 reuses across 32 jobs.

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

## Complete package backend policy

The remaining ports cover all resize filters, Yliluoma, every continuous diffusion
kernel and enabled effect chains. Keep CPU implementations available in the package.
The synchronous CPU interface remains compatible; its GPU counterpart is asynchronous.
Select either through `createDitherette({ engine: 'cpu' | 'gpu', tiling: true | false })`.
The GPU engine shares one artifact/device across the common operation surface and
defaults to GPU diffusion. CPU-only analysis follows the selected fallback policy;
the legacy GPU factories retain their earlier defaults.
Backend, fallback and execution tiling are independent client choices. A GPU-only
request must reject an unavailable stage, including a CPU-only hybrid, instead of
silently returning CPU output. The website consumes these package choices.

Tiling preserves global coordinates, filter halos, alpha coverage and continuous
diffusion state. It must neither restart diffusion at tile boundaries nor turn off
serpentine scanning. Admit all live buffers before allocating; a tile option must
select an actual execution path rather than being accepted and ignored.

The package now exposes CPU row-band selection independently of thread policy,
and GPU facades accept `fallback: 'error'`. GPU `resize` runs all seven filters
with ordinary or coverage carriers, global axis plans, bounded halo tiles and
mapped output. Trilinear keeps its area mip levels and storage-rounding boundaries.
Native checks cover 900 ordinary/coverage cases, all anchors, thin images and
horizontal/vertical tile boundaries. The real browser worker also checks every
filter against CPU and verifies callback recovery. Combined `process` integration
and the remaining GPU methods are the next steps.

Yliluoma now uploads the existing balanced palette-mixture tree through bounded
texture rows and queries it in WGSL. All fifteen metrics retain raw scoring
coordinates, conservative pruning and original enumeration ties; global Bayer
phase and Adaptive output-only placement remain intact. Native 360-case and real
Chromium 120-case fixtures have zero index differences. Four full-size Celeste
cases have 4,424 disagreements across 43,347,200 indices, with largest RGBA byte
difference 238. These are measured disagreements; the GPU result is not byte-exact.
Palette entries and warnings match exactly.

Resolved effects now use a single retained pointwise shader with bounded control
data, avoiding pipeline recompilation when sliders or curve points change. It
evaluates continuous RGB between steps, all nine curve models, periodic splines,
two-input grids, masks and explicit recolour/palette-fit recipes. Whole unmasked
channel chains retain exact byte tables. Automatic image analysis remains the
separate CPU operation described in issue 253; strict GPU calls require its
explicit recipe. Image tiling is independently selectable for effects and resize.

All twelve diffusion kernels now have an opt-in package GPU path. Non-serpentine
scans gather in original raster contribution order through skewed wavefront tiles.
Serpentine keeps a continuous three-row ring while 32 lanes share matching and
disjoint tap/channel updates. Both preserve alpha exclusion, feedback, strength
and output-only Adaptive coverage. A status word rejects non-finite arithmetic
without publishing partial indices. Native cross-tile checks compare the parallel
schedules against an independent serial GPU reference exactly.

Full Celeste GPU/CPU diffusion differs at 3.79–5.94 million of 10.84 million
indices per case as recurrent arithmetic differences propagate through the
texture. Largest per-pixel RGBA differences reach 216; the largest channel-mean
difference in a 32-pixel square is 4.34 bytes. Exact GPU output hashes survived
the cooperative scheduling optimizations. Busy-host single-call diagnostics without progress callbacks put
ordinary non-serpentine Sierra near 0.7 seconds and faithful serpentine near
34 seconds, against about 1.4 seconds on CPU. These are not qualified medians
or speedup claims. CPU diffusion remains the default; `diffusionBackend: 'gpu'`
exposes the device implementation for clients that choose it.

Mia accepted visually similar GPU diffusion on 2026-10-21. `diffusionBackend: 'gpu-approximate'`
runs bands of eight rows in order; each band reads the exact errors of the previous band's
last two rows. Inside a band, 16-pixel row segments run in parallel, each warmed over a
4-pixel halo whose indices it discards, and a hashed shift moves segment edges every band.
One workgroup per segment gathers a row's vertical errors in parallel before one lane scans
it; byte feedback matches through a 16 MiB nearest-slot table built on GPU. Independent
overlapping tiles, with or without minimum-disagreement stitching, left visible seams on
Celeste's sky because their rows never saw the actual row above. Full Celeste with the
Wplace palette differs from CPU at 44–56% of indices, as faithful GPU recurrence already
does; mean 8×8 tone differs by under one byte. Busy-host browser diagnostics put a complete
call near 0.16 s against about 1 s for scalar CPU and 51 s for faithful serpentine.
Matching feedback still searches the palette serially and gains little.
