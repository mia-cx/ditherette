# Palette Matcher Optimization Slice

## Goal

Reduce quantization time with benchmarked, exact-output palette/color matching improvements. This slice should make the next issue #2 optimizations evidence-driven by first splitting quantize timings into actionable sub-stages, then optimizing only the dominant matching costs that preserve byte-for-byte output.

Refs: GitHub issue #2.

## Current Baseline Context

Recent smoke runs after observability showed quantize as the hotspot. Current code shape:

- `src/lib/processing/color.ts`
  - `createPaletteMatcher(colors, mode)` builds typed arrays for visible palette colors.
  - Public matching API returns `{ color, index }` objects via `nearest()` / `nearestRgb()`.
  - Non-RGB modes allocate a `Vector` tuple per `nearestRgb()` via `vectorForRgb(...)`.
  - `oklch` matching pays hue wrapping trig in the inner palette loop.
- `src/lib/processing/quantize.ts`
  - Direct/no-dither and non-vector Bayer/random call `matcher.nearestRgb(r, g, b).index` per output pixel.
  - Error diffusion calls `matcher.nearestRgb(...)`, then uses `match.color` for error calculation.
  - Vector matching paths already cache full-image `Float32Array` vector channels and palette vectors.
  - Palette vector cache is worker-owned and separate from branch image caches.
- `src/lib/benchmark/processing-benchmark.ts`
  - Durable benchmark currently reports coarse `resize`, `quantize`, `previewRender`, `pngEncode`, `total` stages.
- `src/lib/processing/worker-pipeline.ts`
  - App observability records worker `quantize compute` but not sub-stages inside quantize.

## Hypotheses To Test

1. Object-returning matcher calls are measurable overhead in direct/no-dither and non-vector error diffusion.
2. Lazy matcher memoization should cover every nearest-color path, not just RGB byte inputs. Repeated source colors and repeated vector states should reuse exact nearest-index results within the current palette/color-space scope.
3. RGB24 keys are only the byte-RGB entry-point cache. Dynamic/vector color-space paths need their own exact vector-key memoization, not approximate RGB-only shortcuts.
4. Specializing matcher implementations by color space removes repeated mode branches in hot paths without output changes.
5. Existing vector-image caches avoid repeated RGB→vector conversion, but they do not memoize repeated vector→palette nearest searches; this slice should address that gap.

## Non-goals

- Resize → quantize fusion.
- Preview tiling or OffscreenCanvas rendering.
- PNG export workerization/copy reduction.
- Rolling-row error diffusion memory layout.
- Approximate/output-changing LUT modes unless they become explicit UI options in a later PR.
- Removing supported color spaces or resampling modes.

## Deliverables

### 1. Quantize sub-stage instrumentation

Add a lightweight optional diagnostics sink for quantization internals without making production code depend on benchmarks.

Proposed shape:

```ts
export type QuantizeTimingSink = {
	mark(name: QuantizeStageName, start: number): void;
	count?(name: QuantizeCounterName, amount?: number): void;
};
```

Or if lower friction, return optional diagnostics from `quantizeImage(...)` through an extended internal result used by worker/benchmark code only. Avoid broad API churn unless tests justify it.

Required sub-stages/counters:

- `palette prepare`: truncate palette, transparent fallback, matte resolution.
- `matcher build`: `createPaletteMatcher(...)`.
- `palette vector space`: `paletteVectorSpace(...)` including cache hit/miss.
- `color vector image`: build/reuse composited/source vector images.
- `direct loop`: direct/Bayer/random loop body total.
- `error diffusion init`: work-buffer initialization.
- `error diffusion loop`: diffusion loop total.
- `nearest rgb`: count calls to RGB matcher.
- `nearest vector`: count calls to vector matcher.
- `rgb cache hit/miss/set`: once RGB24 cache exists.

Where to surface:

- Worker metrics: append quantize sub-stage timings to `metrics.timings` with names prefixed `quantize ...`.
- Benchmark JSON/CSV: either include sub-stage timings per run or include a `details` object per run/case. Keep existing top-level columns backward-compatible.
- Perf popover: no new table required unless cheap; the existing timing history table can show the additional named stages.

Acceptance criteria:

- Existing benchmarks still run and preserve current top-level stage names.
- App Perf popover still works if detailed quantize metrics are absent.
- Invalid/missing optional diagnostics do not reject a processed image.

### 2. Baseline benchmark evidence

Before optimization commits, capture local ignored benchmark artifacts under `benchmark-results/`.

Minimum cases:

- direct/no dither, sRGB, `useColorSpace=false`.
- direct/no dither, OKLab, `useColorSpace=true`.
- Bayer 8, sRGB, `useColorSpace=false`.
- Bayer 8, OKLab, `useColorSpace=true`.
- Floyd-Steinberg or Sierra, sRGB.
- Floyd-Steinberg or Sierra, OKLab/vector mode.

Source sizes:

- smoke synthetic already in benchmark harness.
- at least one 2–4MP fixture if `benchmark-fixtures/` exists.
- one larger fixture if available and user is willing to run it.

Evidence to save locally:

- baseline JSON/CSV.
- optimized JSON/CSV.
- short comparison table in PR body or comment, not committed artifact files.

### 3. No-allocation matcher API

Add APIs to `PaletteMatcher` that avoid returning new objects per pixel.

Proposed additions:

```ts
export type PaletteMatcher = {
	colors: EnabledPaletteColor[];
	nearest(rgb: Rgb): PaletteMatch;
	nearestRgb(r: number, g: number, b: number): PaletteMatch;
	nearestIndexRgb(r: number, g: number, b: number): number;
	paletteRgbAt(index: number): Rgb;
};
```

Implementation notes:

- Keep `nearest()` / `nearestRgb()` for compatibility and tests.
- Implement `nearestRgb()` via `nearestIndexRgb()` + stable color lookup, not vice versa.
- `paletteRgbAt(index)` should return existing palette RGB object when available or a shared transparent fallback only for paths that already handle transparent defensively. Prefer not to call it for transparent palette entries.
- Replace `.nearestRgb(...).index` in direct/Bayer/random paths with `.nearestIndexRgb(...)`.
- In RGB error diffusion, get index via `nearestIndexRgb(...)`, then get chosen RGB via palette index without allocating `PaletteMatch`.
- Preserve tie-breaking exactly: first visible color with strictly smaller distance wins, same as current `< best` behavior.

Acceptance criteria:

- Existing color and quantize tests pass unchanged or with stronger assertions.
- Add tests that `nearestIndexRgb(...)` equals `nearestRgb(...).index` for every color space and representative RGB samples.
- Add quantize golden/equivalence tests for direct, Bayer, random, and error diffusion outputs before/after API migration.

### 4. Lazy exact matcher memoization for every matching path

Memoization must cover all nearest-color paths lazily. No full LUT precomputation and no approximate/quantized vector keys in this PR. Caches are populated on first miss, bounded, and scoped to the matcher/palette/color-space instance.

#### 4.1 RGB entry-point memoization

This covers every call that starts from byte RGB, including dynamic color spaces, because the matcher instance already bakes in the selected color-space distance function and palette identity.

Design constraints:

- Key: packed `RGB24 = (r << 16) | (g << 8) | b`.
- Scope: matcher instance, which is scoped by palette + enabled colors + color space. No global cache.
- Value: palette index (`number`), not `PaletteMatch`.
- Eligibility: only integer byte inputs. If a path has fractional values, either preserve existing clamping before lookup or bypass the RGB cache.
- Transparency: cache only visible-color matching; alpha threshold behavior remains outside matcher.

#### 4.2 Vector entry-point memoization

This covers existing dynamic color-space paths that already operate on `ColorVector` values:

- `nearestPaletteVectorIndex(...)` in direct vector matching.
- `colorSpaceThresholdIndex(...)` for vector Bayer/random thresholding.
- `quantizeVectorErrorDiffusion(...)` when matching adjusted work-buffer vectors.

Design constraints:

- Introduce a memoizable vector matcher abstraction around `PaletteVectorSpace`, for example:

```ts
type PaletteVectorMatcher = {
	nearestIndex(v0: number, v1: number, v2: number): number;
	nearest(v0: number, v1: number, v2: number): PaletteVector | undefined;
};
```

- Scope: vector matcher instance, keyed by palette vector cache key + color space/distance behavior.
- Value: palette index or visible palette vector ordinal; choose one and make the type explicit so palette index and visible ordinal cannot be confused. Prefer palette index at API boundaries.
- Keying must be exact for the numeric inputs used by the current path:
  - RGB-derived cached vector images can use the exact `Float32Array` channel values; bit-pattern keys are acceptable if they do not coerce values differently before matching.
  - Error-diffusion work-buffer vectors are `Float32Array` values; exact Float32 bit-pattern keys are preferred over stringifying if implementation remains simple.
  - Plain `ColorVector` tuples from `vectorForRgb(...)` may be keyed by exact JS-number tuple only if that does not alter matching semantics. Do not round/quantize.
- No approximate vector buckets, no tolerance-based equality, no perceptual LUTs in this PR.

#### 4.3 Bounds and eviction

- Bounds: fixed conservative entry caps per matcher/cache kind. Start small enough to avoid unbounded photo workloads; tune with benchmarks.
- Eviction: simple insertion-order `Map` LRU or clear-on-cap is acceptable if benchmarked. Prefer the simpler policy unless LRU materially improves hit rate.
- Counters: record RGB and vector memo hit/miss/set/eviction counts in optional quantize diagnostics.
- User controls: none in this PR.

Acceptance criteria:

- All nearest-color loops go through memoizable RGB or vector matcher APIs; no duplicated ad hoc nearest loops remain in quantize hot paths except where explicitly justified.
- Exact byte-for-byte quantize output for all existing and new quantize tests.
- Tests prove repeated RGB inputs hit cache without changing index.
- Tests prove repeated vector inputs hit cache without changing index for at least one cached-vector path and one vector error-diffusion-style Float32 input.
- Benchmark evidence reports memo hit rates and whether cache overhead is acceptable across repeated-color and photographic-like cases.

### 5. Color-space specialization

Refactor `createPaletteMatcher(...)` so mode-specific distance functions are selected once at matcher creation instead of branching per call/per palette entry where possible.

Targets:

- `srgb`: direct squared distance over `Uint8Array` palette channels.
- `weighted-rgb`: redmean weighted distance.
- `weighted-rgb-601` / `weighted-rgb-709`: preselected weights.
- vector spaces (`linear-rgb`, `oklab`, `cielab`): compute source vector once, then simple vector distance loop.
- `oklch`: keep hue-wrap behavior exact; only hoist what is safe.

Acceptance criteria:

- `createPaletteMatcher` remains a single public factory.
- Tie-breaking and output are unchanged.
- No `any` or type erasure.
- Benchmarks include per-color-space comparison.

### 6. Worker/branch cache integration check

If matcher construction remains measurable, consider caching matcher instances or matcher-ready typed arrays in the existing palette vector cache layer. This is optional and only if evidence says matcher build is hot.

Constraints:

- Keep palette-derived caches separate from image branch caches.
- Cache key must include enabled palette color identity and color-space.
- Do not retain a full historical cache tree; bounded active palette-derived cache only.

## Implementation Order

1. Read current benchmark and quantize paths; identify exact call sites for matcher allocation/object churn.
2. Add quantize sub-stage timing/counters with tests for optional metrics schema behavior if schema changes.
3. Run baseline benchmarks and save ignored artifacts.
4. Add `nearestIndexRgb(...)` and migrate call sites.
5. Add equivalence tests and run unit tests.
6. Benchmark no-allocation migration.
7. Add bounded lazy RGB24 memoization behind matcher internals.
8. Add bounded lazy vector memoization behind palette-vector matching internals.
9. Benchmark memoization hit rates and cache overhead across repeated-color and photographic-like cases; tune caps/policy rather than falling back to unbounded caches.
10. Specialize distance functions by color mode if branch overhead is still visible.
11. Run full validation and update PR body with evidence.

## Tests To Add/Update

- `src/lib/processing/color.spec.ts`
  - `nearestIndexRgb` equals `nearestRgb(...).index` across all color spaces.
  - tie-breaking remains first visible color.
  - no visible colors still throws through all public matching APIs.
  - RGB memo hit path returns same index.
  - vector memo hit path returns same palette index for exact repeated vector inputs.
- `src/lib/processing/quantize.spec.ts`
  - direct/no-dither equivalence for optimized matcher path.
  - Bayer/random equivalence for RGB and vector dither modes.
  - error diffusion equivalence for RGB and vector paths.
  - alpha threshold/transparent fallback remains unchanged.
- `src/lib/processing/metrics.spec.ts` or schema tests if quantize sub-stage metrics are added to worker responses.
- Benchmark tests only if durable harness output shape changes.

## Validation Commands

- `pnpm test:unit -- --run`
- `pnpm check`
- `pnpm exec eslint .`
- `pnpm bench:processing -- --profile smoke --iterations 3 --warmups 1 --out benchmark-results/palette-baseline-smoke`
- `pnpm bench:processing -- --profile baseline --iterations 5 --warmups 1 --out benchmark-results/palette-baseline-main` when fixtures/time allow.

## PR Evidence Template

When implementation is ready, add this to the PR body/comment:

```md
### Benchmark evidence

| Case         | Before quantize mean | After quantize mean | Delta | Output guard   |
| ------------ | -------------------: | ------------------: | ----: | -------------- |
| direct sRGB  |                  ... |                 ... |   ... | byte-identical |
| Bayer OKLab  |                  ... |                 ... |   ... | byte-identical |
| Floyd sRGB   |                  ... |                 ... |   ... | byte-identical |
| Sierra OKLab |                  ... |                 ... |   ... | byte-identical |

Artifacts: `benchmark-results/...` local only.
```

## Risks / Watchouts

- Memoization can be slower than raw palette scan for photographic images with many unique colors. Benchmark hit rates and overhead before raising caps.
- `Map` LRU mutation in hot loops can dominate savings; compare LRU against clear-on-cap.
- `compositedRgb(...)` and dither noise may produce values that are already clamped integers in some paths and fractional in others; RGB24 memoization is only valid where exact byte input semantics are preserved.
- Vector memoization must not quantize or round vectors. Exact Float32/number keys only.
- `oklch` hue wrapping is expensive but output-sensitive. Do not approximate it in this PR.
- Palette index vs visible ordinal must stay explicit. Returning visible ordinal by mistake will corrupt indexed output.
- Transparent palette entries are excluded from matching. Preserve that invariant.
- Worker transfer/cached indices are already delicate; do not introduce detached-buffer reuse.

## Done Criteria

- Baseline and after benchmark evidence exists locally and is summarized in PR.
- Optimized paths are byte-for-byte identical to baseline for covered cases.
- The durable benchmark command still works.
- Full checks pass.
- PR may commit durable planning docs and reusable benchmark/study scripts; it must not commit generated `benchmark-results/`, ad-hoc fixture dumps, or scratch-only scripts.
