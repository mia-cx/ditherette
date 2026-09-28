# #307 Add two-input curves

## Summary

Extend the unified `curves` effect with two-input adjustment curves. A two-input curve reads `x` and `x2` from the step’s original pixel, evaluates a control grid, then applies the existing adjustment rule to `y`.

One-input curves keep their current JSON and behavior. The reference defines cyclic and open grid interpolation. Production remains byte-exact through the pointwise colour memo. The package exposes a strict union based on `x2` presence. The website stays untouched.

## Acceptance criteria

- [ ] A curve with `x2` uses `grid`, has no `points`, and has `kind: "adjust"`.
- [ ] `x2` is a valid `ColourChannel` different from `x`.
- [ ] Grids accept 2 to 48 columns and 2 to 16 rows.
- [ ] Open axes start at 0 and end at 1. Hue axes use unique positions in `[0, 1)` and wrap without a duplicate seam.
- [ ] Every adjacent position, including a hue axis’s wrapped seam, has at least the existing `0.001` gap.
- [ ] Grid evaluation runs along `x` first and `x2` second, entirely in `f32`.
- [ ] Both input channels come from the curve step’s original pixel.
- [ ] Grid output is clamped to `[0, 1]` before the existing adjustment rule runs.
- [ ] Hue confidence is the minimum confidence from every hue-valued side among `x`, `x2`, and `y`.
- [ ] A grid containing only exact `0.5` values skips all colour conversion.
- [ ] Existing one-input curve behavior, validation, and optimized table folding remain byte-identical.
- [ ] Reference and production match byte for byte in deterministic and randomized grid comparisons.
- [ ] Package types form a strict one-input or two-input union. Package validation matches Rust validation order and paths.
- [ ] Package documentation and a minor changeset describe the control-grid shape.
- [ ] A new freeze extension replaces only `spec/effects/curves.rs` and `curves.md`.
- [ ] The trusted-base freeze check reports the exact `/approve-freeze sha256:<digest>` line.
- [ ] Root `src/**` remains untouched.

## TODOs

- [x] Extend the frozen reference contract. Add strict one-input and two-input curve variants, `CurveGrid`, grid validation, open and closed-sequence interpolation, original-input selection, result clamping, neutral-grid skipping, and three-sided hue confidence in `spec/effects/curves.rs`. Update `curves.md` with the exact algorithm and invariants. Add focused reference tests for knots, both axis orders, both hue-axis positions, two-knot cyclic interpolation, seam continuity, validation paths, neutrality, ordering, and hue confidence. Record `effects-two-input-curves` as a new freeze extension replacing the two curves spec files. Reference tests and the local freeze guard pass.

- [ ] Implement the direct production equivalent. Keep prepared metadata compact: retain inline one-input splines, store grid curve indices and resolved channels, and use fixed `[f32; 16]` row-result scratch rather than copying maximum grids into `PreparedPointwise`. Preserve the existing all-remap table path. Route every two-input curve through the pointwise path and colour memo. Skip neutral grids during preparation. Extend allocation coverage and deterministic production tests.

- [ ] Prove production equivalence. Add seeded randomized grids covering open and hue axes, `x` and `x2` in either position, mixed colour models, hue outputs, neutral grids, minimum and maximum dimensions, and mixed ordered one-input/two-input lists. Compare reference and production bytes, validation errors, direct `f32` carrier results, and alpha. Full native tests pass with `CARGO_BUILD_JOBS=2` and one heavy job at a time.

- [ ] Cut over the package contract. Add `CurveGrid`, `OneInputCurve`, and `TwoInputCurve`, then export `Curve` as their union. Extend normalization with strict key sets, f32 bounds, dimensions, matrix shape, axis endpoints, cyclic seam gaps, and indexed error paths. Add positive type tests and negative tests for mixed shapes, remap grids, equal inputs, sparse arrays, ragged values, extra keys, invalid seams, and limits. Update the package README and add a minor `ditherette` changeset. Run package type and interface tests without editing root `src/**`.

- [ ] Finish validation and freeze approval. Update production documentation where it describes curves fast paths. Run formatting, focused and full Rust tests, package checks, freeze tests, the local guard, and the trusted-base guard. Report the exact approval digest printed by the trusted guard and confirm the extension contains only the two intended spec replacements.

## Notes

### TODO 1 validation

- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml`: passed.
- `nice env CARGO_BUILD_JOBS=2 cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_effects`: passed, 27 tests.
- `nice env CARGO_BUILD_JOBS=2 cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_grading`: passed, 6 tests.
- `node tools/spec-freeze/extend.mjs effects-two-input-curves "Add two-input control-grid curves"`: recorded exactly two replacements, `curves.rs` and `curves.md`.
- `PATH="$PWD/crates/ditherette-wasm/node_modules/.bin:$PATH" NODE_OPTIONS="--require=/tmp/codex-node-spawn-status.cjs" nice node tools/spec-freeze/guard.mjs`: passed. The temporary shim converts this sandbox's false `EPERM` after a child exits with status 0 into the captured successful result, and injects `CARGO_BUILD_JOBS=2` for Cargo subprocesses. The guard returned frozen artifact `sha256:a5f90b9b3b864c4826748646818bbe1c903de958811f4c5af3345082cf5ad47d`.

### Exact JSON shape

One-input curves keep the existing shape:

```json
{
  "kind": "adjust",
  "x": { "model": "hsl", "channel": "hue" },
  "y": { "model": "oklch", "channel": "chroma" },
  "points": [[0, 0.5], [0.5, 0.8], [1, 0.5]]
}
```

A two-input curve has exactly `kind`, `x`, `x2`, `y`, and `grid`:

```json
{
  "kind": "adjust",
  "x": { "model": "hsl", "channel": "hue" },
  "x2": { "model": "oklch", "channel": "lightness" },
  "y": { "model": "oklch", "channel": "chroma" },
  "grid": {
    "columns": [0, 0.25, 0.5, 0.75],
    "rows": [0, 0.5, 1],
    "values": [
      [0.5, 0.7, 0.5, 0.3],
      [0.5, 0.9, 0.5, 0.2],
      [0.5, 0.6, 0.5, 0.4]
    ]
  }
}
```

`values[row][column]` corresponds to `rows[row]` on `x2` and `columns[column]` on `x`. Values are inclusive from 0 through 1. Exactly `0.5` is neutral.

The package shape is:

```ts
export interface CurveGrid {
  readonly columns: readonly number[];
  readonly rows: readonly number[];
  readonly values: readonly (readonly number[])[];
}

export interface OneInputCurve {
  readonly kind: 'remap' | 'adjust';
  readonly x: ColourChannel;
  readonly y: ColourChannel;
  readonly points: CurvePoints;
  readonly x2?: never;
  readonly grid?: never;
}

export interface TwoInputCurve {
  readonly kind: 'adjust';
  readonly x: ColourChannel;
  readonly x2: ColourChannel;
  readonly y: ColourChannel;
  readonly grid: CurveGrid;
  readonly points?: never;
}

export type Curve = OneInputCurve | TwoInputCurve;
```

### Evaluation rule

All decoded numbers and arithmetic are `f32`.

For each curve, retain the step’s `source` and accumulated `current`:

1. Convert `source` to `x.model`. Read `x_value`.
2. Convert `source` to `x2.model`. Read `x2_value`. Reusing coordinates is valid when both models match.
3. For each row `r` in ascending order, evaluate the columns spline through `values[r]` at `x_value`. Store the result in `row_results[r]`.
4. Evaluate the rows spline through `row_results` at `x2_value`.
5. Clamp that result with `c = result.clamp(0.0, 1.0)`.
6. Start `w = 1.0`. If `x` is hue, set `w` to its source hue confidence.
7. If `x2` is hue, set `w = w.min(x2_source_confidence)`.
8. Return `current` immediately when `w == 0.0`.
9. Convert `current` to `y.model`.
10. If `y` is hue, set `w = w.min(y_current_confidence)`. Return `current` when it becomes zero.
11. Apply the existing adjustment:

```text
hue:    y = (y + w * (c - 0.5)).rem_euclid(1.0)
chroma: y = y * (1.0 + w * (2.0 * c - 1.0))
other:  y = y + w * (c - 0.5)
```

12. Convert the edited Y coordinates back to the carrier. This becomes `current`.

Do not clamp the edited Y coordinate before its model conversion. Each later curve again reads both inputs from `source`.

### Cyclic interpolation

An open axis uses the existing ordinary Fritsch–Butland spline.

For cyclic positions `p[0..n]`, treat the sequence as closed:

```text
p[n] = p[0] + 1
v[n] = v[0]
```

Each knot tangent uses the wrapped previous and next secants with the existing `tangent` operation. Evaluation wraps the input with `rem_euclid(1.0)`. Inputs below `p[0]` use the seam segment from the last knot to `p[0] + 1`.

Two positions are sufficient. They form two closed segments. Opposing secants produce zero knot tangents under the existing Fritsch–Butland rule, so no three-position exception is needed.

### Validation rules and paths

Validation follows field order: `kind`, `x`, `x2`, `y`, then `grid`.

- A two-input `kind` other than `adjust` fails at `effects.i.curves.k.kind`.
- An invalid second channel fails at `effects.i.curves.k.x2.model` or `.x2.channel`.
- `x2 == x` fails at `effects.i.curves.k.x2`.
- Missing, extra, or mixed `points` and `grid` fields fail the strict curve shape.
- Invalid column count fails at `effects.i.curves.k.grid.columns`.
- Invalid row count fails at `effects.i.curves.k.grid.rows`.
- Invalid column positions fail at `effects.i.curves.k.grid.columns.j`.
- Invalid row positions fail at `effects.i.curves.k.grid.rows.j`.
- An open axis requires its first position to equal 0 and its last to equal 1.
- A hue axis accepts finite positions in `[0, 1)`. It has no endpoint requirement and no duplicate seam.
- Adjacent gaps use the existing f32 `MIN_GAP`.
- A cyclic seam gap is `1.0 - last + first`. A gap below `MIN_GAP` fails at the first position, `.columns.0` or `.rows.0`.
- `values.length` must equal `rows.length`; otherwise validation fails at `effects.i.curves.k.grid.values`.
- Each value row must contain exactly `columns.length` entries; otherwise it fails at `effects.i.curves.k.grid.values.r`.
- A non-finite or out-of-range value fails at `effects.i.curves.k.grid.values.r.c`.
- Existing one-input point validation and duplicate hue seam rules stay unchanged.
- TypeScript validation rounds every number with `Math.fround` before comparisons.

### Freeze mechanics

Run:

```sh
node tools/spec-freeze/extend.mjs effects-two-input-curves "Add two-input control-grid curves"
```

The appended extension replaces:

```text
crates/ditherette-wasm/src/spec/effects/curves.rs
crates/ditherette-wasm/src/spec/effects/curves.md
```

It must not replace the existing `effects-curves` entry or change the freeze tools. Run the local guard first. Then run the trusted parent checkout’s guard against this worktree. Copy its exact `/approve-freeze sha256:<digest>` output into the implementation report.

### Production fast paths

- The table path remains limited to lists containing only one-input sRGB or linear-RGB remaps.
- Any two-input curve makes the step pointwise.
- The existing colour memo covers repeated input colours without a compiled 2D LUT.
- Neutral grids are removed from prepared work before any model conversion.
- Grid evaluation uses fixed row scratch and no per-pixel heap allocation.
- Maximum grids must not inflate every `PreparedPointwise` entry with inline `48 × 16` storage.
- Mixed lists retain source selection and accumulated output order exactly.

### Website exclusions

The package union will expose assumptions in the current one-input editor. Leave these root files unchanged and hand them to the grid-editor worker:

- `src/lib/effects/catalog.ts`: return a one-input subtype from `neutralCurve`.
- `src/lib/effects/pick.ts`: accept a one-input curve instead of the full union.
- `src/lib/effects/pick.spec.ts`: retain the one-input fixture type.
- `src/lib/stores/effects.spec.ts`: narrow the edited curve before spreading `points`.
- `src/routes/components/effects/CurvesEditor.svelte`: branch on `x2` before reading or writing `points`.
- `src/routes/components/effects/CurvesEditor.svelte.spec.ts`: narrow before direct `points` assertions.

`CurveGraph.svelte` consumes `CurvePoints` directly and should not need a package-type fix.

### Open questions

None. The existing cyclic helper and two-input prototype establish that two unique hue positions are sufficient.
