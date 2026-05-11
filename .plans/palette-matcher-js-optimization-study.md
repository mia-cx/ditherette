# Palette Matching & Dithering JS/TS Optimization Study

## Goal

Exhaust every exact-output JavaScript/TypeScript optimization opportunity in Ditherette palette matching and dithering before considering Rust/WASM. The main bottleneck is not resize, preview, PNG, or general pipeline overhead: it is repeated nearest-palette lookup under dithered color transforms.

This plan uses Celeste box art at native source size only:

```text
source: benchmark-fixtures/Celeste_box_art_full.png
size:   2600×4168 = 10,836,800 pixels
resize: none
scope:  quantize / palette match / dither only
```

At ~64 palette colors, a brute-force exact nearest-color pass costs:

```text
10.84M pixels × 64 colors ≈ 693.6M candidate distance evaluations per case
```

That is the problem to eliminate, prune, cache, or make cheaper.

## Non-goals

- Rust/WASM porting. Defer until JS/TS has been fully studied.
- Resize benchmarking or scale permutation.
- Preview rendering or PNG encoding.
- Approximate/output-changing quantization unless behind a future explicit option.
- Removing supported color spaces or dither modes.
- Parallelizing exact error diffusion in a way that changes output order/state.

## Benchmark Invariant

All study and integration benches must:

1. Decode Celeste once.
2. Use the decoded source `ImageData` directly.
3. Do no resize and no resize cache lookup.
4. Run only quantize/dither/palette matching.
5. Validate output with deterministic checksums when comparing variants.
6. Report build time separately from loop time.
7. Report candidate evaluations, cache hits/misses/collisions, and memory estimates.

## Required Benchmark Commands

### Dedicated study harness

Add:

```text
scripts/benchmark-palette-study.mjs
src/lib/benchmark/palette-study.ts
```

Target command:

```bash
pnpm bench:palette-study --image benchmark-fixtures/Celeste_box_art_full.png --iterations 10 --warmups 1
```

The harness should expose study filters:

```bash
pnpm bench:palette-study --study direct-byte-rgb
pnpm bench:palette-study --study bayer-threshold-vector --dither bayer-16 --color-space weighted-rgb-601
pnpm bench:palette-study --study diffusion-trace --dither sierra --color-space srgb
pnpm bench:palette-study --study matcher --matcher kd-tree --color-space oklab
```

### Existing processing harness extension

Add no-resize mode:

```bash
pnpm bench:processing --image benchmark-fixtures/Celeste_box_art_full.png --no-resize --dither --color-space --iterations 10 --warmups 1
```

Expected matrix:

```text
9 dither modes × 8 color spaces = 72 cases
```

No scale dimension. No resize dimension.

## Study Harness Output Schema

Each study row should include:

```text
study
source
pixels
paletteColors
dither
colorSpace
variant
buildMs
loopMs
totalMs
candidateEvaluations
queries
uniqueKeys
cacheHits
cacheMisses
cacheCollisions
cacheSets
cacheBytes
tableBytes
workBytes
checksum
matchesBaseline
notes
```

Also emit JSON/CSV under `benchmark-results/palette-study-<timestamp>/`.

## Core Datasets To Generate Once Per Run

### 1. `direct-byte-rgb`

One byte RGB query per source pixel after alpha handling.

Use for:

- direct/no-dither matching
- dense RGB memo variants
- RGB24 LUT variants
- byte-entry OKLab/CIELAB/OKLCH matching

### 2. `bayer-additive-byte-rgb`

For non-vector/additive Bayer paths, generate clamped byte RGB queries for:

```text
bayer-2
bayer-4
bayer-8
bayer-16
```

Use for:

- RGB memo/cache variants
- additive RGB dither loop specialization

### 3. `bayer-threshold-vector`

For vector Bayer paths, generate exact query descriptors:

```text
thresholdIndex
r
g
b
source0/source1/source2 when cached vector path applies
```

Use for:

- threshold-aware caches
- KD-tree / VP-tree / BVH matchers
- unique-key prepass
- threshold table layouts

### 4. `random-vector`

Generate deterministic random dither vector query stream with the production seed.

Use for:

- PRNG overhead studies
- vector matcher studies
- random cache hit-rate reality check

### 5. `diffusion-trace`

Record exact vector/RGB query stream from one baseline error-diffusion run, then replay against matcher variants.

Use for matcher speed only. Final integration still required because diffusion state depends on selected output.

### 6. `diffusion-kernel-only`

Bypass palette matching with a fixed/fake nearest result to isolate diffusion scatter/update overhead.

Use for:

- kernel unrolling
- rolling-row work buffers
- serpentine loop specialization

### 7. `cache-study`

Feed recorded query streams through cache variants only.

Report:

```text
hit rate
miss rate
collision rate
lookup ns/query
memory bytes
```

## Instrumentation Required Before Optimization

Add counters to quantize and study matchers:

```text
candidate evaluations
nearest rgb calls
nearest vector calls
rgb memo hit/miss/set/eviction
vector memo hit/miss/set/eviction
threshold cache hit/miss/set/collision
threshold unique keys
matcher build bytes
threshold table bytes
dense cache bytes
vector image bytes
work buffer bytes
```

Current `vec matches 10.8M` is insufficient. We need to know how many palette candidates were actually evaluated.

## Optimization Plan

### Phase 1 — Low-risk hot-loop cleanup

#### 1.1 Dense RGB24 memo for all byte-RGB entry paths

Current underuse: dense memo is enabled only for `weighted-rgb`. Enable for all byte RGB entry paths when memory budget allows:

```text
srgb
linear-rgb
weighted-rgb
weighted-rgb-601
weighted-rgb-709
oklab
cielab
oklch
```

Exact because the matcher instance already bakes in palette + color space.

Benches:

```text
direct-byte-rgb × all color spaces
bayer-additive-byte-rgb × RGB-like modes
rgb diffusion weighted-rgb
```

Metrics:

```text
rgb hits/misses
candidate evaluations avoided
16MB cache cost vs speedup
```

#### 1.2 Inline compositing in hot loops

Remove per-pixel object creation/destructuring:

```ts
compositedRgb({ r, g, b }, alpha, alphaMode, matte)
```

Replace with scalar branches specialized by alpha mode.

Benches:

```text
bayer-16 srgb
bayer-16 weighted-rgb-601
random oklab
```

#### 1.3 Specialize alpha preserve mode

Most benchmark cases preserve alpha. Split loops for:

```text
alphaMode=preserve
alphaMode=premultiplied
alphaMode=matte
```

Avoid compositing checks in preserve-mode hot loops.

#### 1.4 Specialize `placement: everywhere`

Current benchmark settings use placement everywhere, but production loops still call `placementMask`. Branch once before the loop and use no-mask loops.

Applies to:

```text
Bayer
random
vector diffusion
RGB diffusion
```

Benches:

```text
bayer-16 weighted-rgb-601
random oklab
sierra srgb
floyd weighted-rgb
```

#### 1.5 Remove hot array/object returns

Avoid:

```ts
cachedVectorAt(...) -> [v0, v1, v2]
vectorForRgb(...) tuple allocation in per-pixel threshold paths
nearest(...) returning PaletteVector object where ordinal/index is enough
paletteRgbAt(...) returning object in RGB diffusion hot loop
```

Add scalar APIs:

```ts
nearestOrdinal(v0, v1, v2)
indexForOrdinal(ordinal)
vector0ForOrdinal(ordinal)
vector1ForOrdinal(ordinal)
vector2ForOrdinal(ordinal)
rgbChannelsForPaletteIndex(index)
```

Benches:

```text
floyd oklab
sierra srgb
sierra weighted-rgb-709
bayer-16 oklab
```

### Phase 2 — Distance table and scan micro-architecture

#### 2.1 Float32 distance/threshold tables

Current tables use `Float64Array`. Study `Float32Array` for:

```text
srgb distance tables
linear-rgb distance tables
weighted-rgb-601/709 distance tables
threshold vector tables
```

Validation: checksum must match baseline. If ties flip in weighted/linear modes, keep Float64 only for affected modes.

#### 2.2 Table layout variants

Study:

```text
SoA: rTerms[], gTerms[], bTerms[]
AoS by palette ordinal: [r,g,b,r,g,b]
value-major layout: value × ordinal
single summed partial table variants
```

Goal: reduce cache misses and pointer chasing during 64-color scans.

#### 2.3 Partial-distance early exit

For Euclidean metrics:

```ts
d0 = ...
if (d0 >= best) continue
d1 = ...
if (d0 + d1 >= best) continue
d2 = ...
```

For OKLCH:

```text
compute L/C lower bound first
compute hue only if still competitive
```

Bench:

```text
direct oklch
random oklch
bayer oklch
diffusion oklch
```

#### 2.4 Candidate ordering

Seed `best` with likely candidates before full scan:

```text
previous pixel winner
left pixel winner
above pixel winner
cache result
palette frequency order from prior pass
```

Tie rule must remain exact:

```ts
if (distance < best || (distance === best && ordinal < winner))
```

Bench:

```text
direct
Bayer
random
diffusion
```

### Phase 3 — Cache strategy study

#### 3.1 Dense RGB24 variants

Study:

```text
Uint8Array value+1
Uint16Array value+1
Uint8Array + valid bitset
two-level lazy 256-page cache
Map fallback
clear-on-cap Map
```

Bench:

```text
direct srgb
direct weighted-rgb-601
bayer additive weighted-rgb
RGB diffusion weighted-rgb
```

#### 3.2 Threshold-aware Bayer cache

Exact key:

```text
thresholdIndex + rgb24
```

Variants:

```text
direct-mapped 2^18..2^24
2-way set associative
4-way set associative
per-threshold direct cache
per-threshold set-associative cache
open-addressed typed hash table
```

Bench:

```text
bayer-2 weighted-rgb-601/709
bayer-4 weighted-rgb-601/709
bayer-8 weighted-rgb-601/709
bayer-16 weighted-rgb-601/709
```

Required metrics:

```text
hits
misses
collisions
unique keys
candidate evaluations avoided
cache bytes
```

#### 3.3 Unique-key prepass for Bayer

Bayer pixels are independent. Exact strategy:

```text
1. compute key per pixel: thresholdIndex + rgb24 or thresholdIndex + vector source key
2. unique keys
3. match each unique key once
4. fill output from result table
```

Variants:

```text
JS Map
per-threshold JS Map
Uint32Array radix sort
chunked radix sort
per-threshold buckets + sort
```

Bench:

```text
bayer-16 weighted-rgb-601
bayer-16 weighted-rgb-709
bayer-16 srgb
bayer-8 oklab
```

#### 3.4 Matcher reuse across repeated runs

Cache built matchers by:

```text
palette hash
enabled colors
color space
dither algorithm
bayer size
strength
alpha/matte settings when relevant
```

Use bounded LRU. Report memory.

Bench:

```text
10 repeated no-resize Celeste quantize runs
same palette / same color space
same palette / changing dither
changing color space
```

### Phase 4 — Exact nearest-neighbor algorithms

#### 4.1 KD-tree / VP-tree / ball-tree palette matcher

Applies to Euclidean spaces:

```text
srgb
linear-rgb
weighted-rgb-601
weighted-rgb-709
oklab
cielab
```

Not directly OKLCH because hue distance is circular/non-Euclidean.

Study variants:

```text
brute force typed scan
KD-tree median split
VP-tree
ball tree / bounding sphere
small BVH / AABB tree
```

Metrics:

```text
visited candidates/query
branch count proxy
loop ms
build ms
checksum
```

Datasets:

```text
direct-byte-rgb
bayer-threshold-vector
random-vector
diffusion-trace
```

#### 4.2 Voronoi candidate verification

For Euclidean spaces, candidate A is nearest if it satisfies pairwise half-space tests against all other palette colors.

Use guesses:

```text
last winner
left winner
above winner
cache winner
frequency winner
```

If verification passes, skip full distance scan. Else fallback.

Bench:

```text
direct
Bayer
random
diffusion trace
```

#### 4.3 Conservative grid classifier

Build a 3D grid over vector space:

```text
16³
32³
64³
adaptive subdivision
```

For each cell, prove whether one palette color is nearest for the entire cell. If ambiguous, fallback to scan/tree.

Applies to Euclidean spaces. Exact only with conservative certification.

Metrics:

```text
certified-cell rate
fallback rate
lookup ms
build ms
memory
checksum
```

#### 4.4 Full/lazy RGB24 LUT

Exact for byte-RGB entry with fixed palette + color space:

```text
Uint8Array(16,777,216) ≈ 16MB
```

Study:

```text
eager full build
lazy 256-page build
background build across idle slices
reuse across repeated runs
```

Metrics:

```text
build time
lookup time
break-even render count
memory
```

### Phase 5 — Bayer-specific optimization

#### 5.1 Precompute threshold deltas

Precompute per Bayer size/color-space:

```text
thresholdIndex -> threshold
thresholdIndex -> vector delta0/delta1/delta2
thresholdIndex -> additive RGB noise
row -> threshold row offset
x mask instead of x % size
```

#### 5.2 Specialized Bayer loops by size

Separate loops:

```text
bayer-2:  x & 1
bayer-4:  x & 3
bayer-8:  x & 7
bayer-16: x & 15
```

Avoid generic `%` and repeated matrix lookup cost.

#### 5.3 Bayer worker parallelism

Bayer has no cross-pixel dependency. Study worker parallelism:

```text
2 workers
4 workers
hardwareConcurrency - 1
```

Measure transfer overhead:

```text
copy chunks
transfer output chunks
SharedArrayBuffer if headers/runtime allow
```

Exact output should be easy because rows are independent.

### Phase 6 — Random dither optimization

#### 6.1 Inline Mulberry32

Remove closure call per pixel while preserving sequence exactly.

Bench:

```text
random srgb
random oklab
random weighted-rgb-601
```

#### 6.2 Precompute random noise

Study `Float32Array` noise generation once vs inline PRNG.

Likely useful for study/repeated bench, uncertain for production.

#### 6.3 Confirm cache futility

Measure unique vector keys and exact cache hit rate. If hits are near zero, remove random vector cache attempts from production paths.

### Phase 7 — Error diffusion optimization

#### 7.1 Hand-unroll kernels

Replace tuple iteration/destructuring with specialized forward/reverse loops:

```text
floydForward / floydReverse
sierraForward / sierraReverse
sierraLiteForward / sierraLiteReverse
```

Bench:

```text
floyd srgb
floyd oklab
sierra srgb
sierra oklab
sierra-lite oklab
```

#### 7.2 Rolling-row work buffers

Current full work buffer:

```text
Float32Array(width * height * 3)
```

At full Celeste:

```text
10.84M × 3 × 4 ≈ 130MB
```

Study rolling buffers:

```text
Floyd: current + next rows
Sierra: current + next + next2 rows
Sierra-lite: current + next rows
```

Preserve exact traversal and update order.

#### 7.3 Avoid palette RGB object lookup

Expose palette RGB typed arrays or scalar getters for RGB diffusion:

```text
paletteRedByIndex
paletteGreenByIndex
paletteBlueByIndex
```

Avoid `paletteRgbAt(index)` object lookup per pixel.

#### 7.4 Diffusion matcher acceleration

Use best matcher from Phase 4 for vector diffusion. This is likely mandatory because exact cache hit rate is poor.

### Phase 8 — Color conversion leftovers

Not the primary bottleneck, but study after matcher fixes.

#### 8.1 Lazy vector cache for expensive byte RGB conversions

For OKLab/CIELAB/OKLCH byte RGB:

```text
rgb24 -> vector
```

Variants:

```text
direct-mapped vector cache
set-associative vector cache
lazy page cache
Map
```

Avoid full RGB24 vector LUT unless reused; full Float32 vector LUT is ~192MB.

#### 8.2 OKLCH lower-bound optimization

In OKLCH matcher:

```text
compute L/C lower bound first
skip hue calculation when already worse than best
```

Exact.

### Phase 9 — Integration benches

Focused no-resize Celeste bench after each accepted optimization:

```bash
pnpm bench:processing --image benchmark-fixtures/Celeste_box_art_full.png --no-resize --dither --color-space --cases direct-or-focused-case-list --iterations 10 --warmups 1
```

Full no-resize Celeste matrix:

```bash
pnpm bench:processing --image benchmark-fixtures/Celeste_box_art_full.png --no-resize --dither --color-space --iterations 10 --warmups 1
```

Full unit validation:

```bash
pnpm test:unit -- --run
```

## Acceptance Gates For Each Optimization

Keep a change only if:

1. Output checksum matches baseline for target cases.
2. Isolated study improves target path by at least 10%, or enables a later larger win.
3. Full no-resize Celeste bench does not regress adjacent cases by more than 5% unless justified.
4. Memory cost is reported and bounded.
5. Code complexity is contained behind matcher/cache abstractions.
6. Benchmark artifacts identify before/after JSON paths.

## Expected High-Impact Order

1. Add no-resize processing mode and dedicated palette-study harness.
2. Add candidate/collision/memory counters.
3. Enable dense RGB memo for all byte-RGB paths.
4. Remove hot-loop object/array allocation.
5. Specialize alpha preserve and placement everywhere loops.
6. Switch safe tables to Float32 after checksum validation.
7. Study threshold cache variants and Bayer unique-key prepass.
8. Study KD-tree/VP-tree/BVH exact vector matching.
9. Implement best exact vector matcher if it beats brute force.
10. Hand-unroll diffusion kernels.
11. Study rolling-row diffusion buffers.
12. Study Bayer worker parallelism.
13. Study RGB24 LUT reuse/break-even.
14. Only after all JS/TS wins are exhausted, revisit Rust/WASM.

## PR Evidence Template

```md
### Native Celeste palette/dither benchmark

Source: `benchmark-fixtures/Celeste_box_art_full.png` 2600×4168, no resize.

| Case | Before q loop | After q loop | Candidate evals before | Candidate evals after | Cache hits | Memory delta | Checksum |
|---|---:|---:|---:|---:|---:|---:|---|
| none / srgb | ... | ... | ... | ... | ... | ... | match |
| bayer-16 / weighted-rgb-601 | ... | ... | ... | ... | ... | ... | match |
| random / oklab | ... | ... | ... | ... | ... | ... | match |
| sierra / srgb | ... | ... | ... | ... | ... | ... | match |

Artifacts local only: `benchmark-results/...`
```

## Risks / Watchouts

- Direct-mapped caches can look good on small benches and collapse under real key distributions. Always report collisions.
- Threshold tables for Bayer-16 can exceed 100MB if Float64 and 64-color palette are used. Cache/build memory must be explicit.
- Vector diffusion exact cache hit rate is likely near zero; algorithmic nearest-neighbor acceleration is more promising than memoization there.
- KD-tree/VP-tree branch overhead may beat pruning for 64 colors; benchmark before adopting.
- Float32 tables can flip near ties in weighted/linear/vector spaces; checksum everything.
- Error diffusion order is output-defining. Rolling-row buffers must preserve update order exactly.
- Worker parallelism is safe for Bayer/direct/random, not for exact global error diffusion.
- Palette index and visible ordinal must stay distinct.
- Transparent palette entries must remain excluded from nearest-color matching.
