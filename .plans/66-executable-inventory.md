# S25 executable benchmark inventory

The frozen spec inventory remains unchanged. This document records production benchmark coverage after S25 integration.
`quantize_adapters::executable_inventory_resolves_every_matching_component_and_frozen_oracle` checks registrations, oracle targets, formats, and source paths.

## Complete quantize calls

Native subject `candidate:quantize:request:prepared` uses `prod::quantize::quantize` with a borrowed source and unlimited developer budget.
Its frozen oracle is `spec:quantize:request:v1`.
Every timed native iteration includes preparation, allocation, matching, result construction, and owned result destruction.
Native verification runs the fixed deterministic operation before and after timing. It does not observe every timed result.
The public subject calls the installed package's unchanged `quantize(request)` method once per latency sample.
Public verification checks every durable result outside timers, including warmup, and retains the first distinct output on instability.
No application cache exists in either scope. Neither scope includes module initialization.

| Matching policy | Coordinate space | Metric family |
| --- | --- | --- |
| srgb-euclidean | sRGB | Euclidean squared |
| linear-rgb-euclidean | Linear RGB | Euclidean squared |
| oklab-euclidean | Oklab | Euclidean squared |
| cielab-euclidean | CIELAB | Euclidean squared |
| ycbcr-euclidean | Full-range BT.601 YCbCr | Euclidean squared |
| srgb-compuphase | sRGB | CompuPhase squared |
| srgb-rec601 | sRGB | Rec.601 weighted squared |
| srgb-rec709 | sRGB | Rec.709 weighted squared |
| oklch-euclidean | Oklch | Coordinate Euclidean squared |
| oklch-circular-hue | Oklch | Geometric-mean chroma chord squared |
| oklch-hue-arc | Oklch | Minimum-chroma wrapped arc squared |
| cielab-ciede2000 | CIELAB | CIEDE2000 delta E |
| cielch-euclidean | CIELCH | Coordinate Euclidean squared |
| cielch-circular-hue | CIELCH | Geometric-mean chroma chord squared |
| cielch-hue-arc | CIELCH | Minimum-chroma wrapped arc squared |

The settings identity preserves palette order and duplicates, transparent entries, alpha mode, f64 threshold, and matching tag.
Exact output checks include indices, normalized palette RGBA, transparent index, and every warning code/message.
`quantize_conformance` emits 47 frozen fixtures covering 15 policies, three alpha modes, transparent-only input, and palette truncation.
The installed-package browser fixture checks all 47 in primed and fresh instances. Running it requires coordinator-prepared artifacts.

## Component controls

Seven `prod:color:<space>:packed-forward` subjects use the existing packed converter.
Spaces are `srgb`, `linear-rgb`, `oklab`, `oklch`, `cielab`, `cielch`, and `ycbcr`.
Each targets `spec:color:<space>:f32-roundtrip-v1`; exact f32 coordinates and byte alpha determine conformance.
Frozen inverse rendering is diagnostic and untimed. S25 measures only the two new cylindrical forward controls.

Seven `prod:metric:<family>:cyclic-scores-v1` subjects target corresponding `spec:metric:<family>:cyclic-scores-v1` oracles.
Families are `euclidean`, `chord`, `arc`, `compuphase`, `rec601`, `rec709`, and `ciede2000`.
Each source pixel scores against its cyclic successor, including the last pixel against the first.
Frozen forward conversion prepares both coordinates outside timing. Euclidean/weighted use sRGB; chord/arc use Oklch; CIEDE2000 uses CIELAB.
One native iteration processes all pairs into preallocated f32 scores with input/output black-box barriers.
The scope includes batch iteration and scalar function dispatch, but excludes conversion, pair preparation, and output allocation.
These unchanged functions provide component coverage, not an acceleration claim.

Scores serialize as `format: scores` and `score_bits: u32[]`. They never masquerade as image bytes.
Count must equal fixture width × height and every decoded score must be finite.
Any bit difference fails exact conformance, including signed zero. Reports retain differing-score counts and numeric deltas without PNGs.

## Fresh experiment declarations

`matching_integration_plan` accepts `existing-native`, `existing-public`, `new-native`, or `new-public`.
Every case uses the S24 128×96 varied-RGBA fixture and complete quantize cases retain its 64-entry palette.
The existing-mode group uses the delivered S24 parent as accepted. The new-mode group uses all-mode baseline `0085972a05a3dbdbbef6d47351d6e37bdd8625d2`.
Both compare against the proposed dispatch candidate in separate prepared experiments.
Five existing native/public controls across three browsers consume 80 workers.
Ten new native/public controls, two native conversions, and seven native score controls consume 196 workers.
Total ceiling is 276 workers, two AB/BA pairs, 20 samples, 50 ms warmup, and 250 ms measurement cap.
Full CIEDE2000 quantize alone has a predeclared 10,000 ms cap to accommodate the minimum valid sample count.
Its metric-only score control retains 250 ms. The exception changes neither workers nor sample ceilings and makes no performance claim.
The paired comparator retains its existing minimum-five-sample rule, exactness gate, and regression/noise checks.

## S24 artifact compatibility

The delivered S24 worker/package can be reused for the five existing modes. Old timing samples cannot.
The retained S24 source is `f4b90ecfcde63531fb992cf87ebda04d4e373032`, with unchanged processing bytes at PR113 head `884a8868`.
Current protocol changes add enum variants and an optional score report. Existing request/result fields and normalization remain unchanged.
The ignored `existing_modes_roundtrip_actual_s24_requests_and_results_unchanged` fixture reads coordinator-retained artifacts without executing workers.
It checks all five native and three-browser identities, RGBA bytes, settings, scopes, and subject IDs against newly generated cases.
All 20 recorded candidate requests and results round-trip unchanged. No adapter overlay or feature is required.
Run that fixture with `DITHERETTE_S24_RETAINED_ROOT` pointing to the coordinator's `s24-quantize-attempt-02` directory.
Artifact identities must retain their actual source/binary digests. Fresh preparation must bind the current browser collector snapshots.
