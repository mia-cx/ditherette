# Diffusion row addressing

The scalar diffusion loop recomputed ring-buffer offsets and source alpha rows for each tap.
It now resolves three work offsets and optional alpha-row references once per output row.
The change adds no heap allocation. Tap order, arithmetic, alpha checks, and errors remain unchanged.

## Native selection

Accepted revision `0ebcae59c1404ce027633c4e5b2917055b77ddc7` includes adaptive row reuse.
Candidate revision `98240c4f42e45552c7b8978f7118bc7f50fd6cc0` adds row addressing only.
All 35 cases pass the existing native gate and match frozen output exactly.
The run completes 140 serial workers and 11,200 samples.
The unchanged plan uses two alternating pairs, 80 samples per worker, 250 ms warmup, and 1,000 ms measurement budget.

Candidate/accepted native median ratios are 0.917–0.994 across the 24 non-adaptive diffusion controls.
Adaptive diffusion ratios are 0.960–1.007. Unchanged field controls are 1.006–1.019.

## WebKit comparison

A fresh before/after run uses the retained scalar packages, Celeste at 2600 by 4168, and six samples per state.
Both trials compare historical JS without changing its implementation.
Cold means fresh worker and processor; import and initialization are excluded. Compilation caches may persist.
Warm means recomputing the requested size after priming different dimensions.
Palette-edit calls prime a rotated palette at the same dimensions.

| Recipe | Previous warm Wasm ms | Candidate warm Wasm ms | Candidate-run JS ms |
|---|---:|---:|---:|
| 50% Floyd-Steinberg, sRGB byte feedback | 609.5 | 589.5 | 383.0 |
| 10% adaptive diffusion, sRGB matching | 28.0 | 28.0 | 17.5 |
| 10% adaptive diffusion, weighted RGB 601 matching | 28.0 | 27.0 | 18.5 |

The larger case improves 3.28%, meeting the declared 3% candidate threshold.
Its cold time changes from 650 to 631.5 ms; palette-edit time changes from 605 to 583 ms.
All three before/after PNGs match exactly. Repeated outputs are also exact within each trial.
Six samples are a diagnostic comparison, not a confidence-bound release gate.
Every listed recipe still misses the JS target. This is a small retained improvement, not a claim that WebKit overhead is solved.

The isolated candidate passes 13 Rust diffusion tests, including the frozen matrix, and its built scalar ABI check.
The delivery stacks above the independently measured cache-aware nearest policy. The isolated browser comparison excludes that policy to measure row addressing alone.
The joined runtime passes 476 native tests with `bench-subjects`, 21 private Wasm tests, and scalar/threaded builds.
Two inherited Yliluoma cache-hit assertions required a test-boundary snapshot implementation. That seven-line fixture correction changes no runtime behavior and preserves failed-copy injection.
Final runtime artifacts identify source `753bfebe338a48e9204ff8e47194afe89a6eb775`; the later fixture commit changes tests only.

Native requests/results, frozen comparisons, executable provenance, browser samples, manifests, scripts, image equality records, and final validation logs are retained in the [evidence archive](diffusion-row-addressing-evidence.tar.xz).
Its SHA-256 is `a59c28bc2e908b6f98682a14d68bca01ce6a02bc8da88e4ef5b83cf52f86cf79`.
Compiled artifacts and PNGs remain outside compiler targets.
