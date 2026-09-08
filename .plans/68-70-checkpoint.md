# S27 through S29 integration checkpoint

S26 PR #115 is open and unmerged. Coordinator `25c1dec8` contains its accepted
implementation and all restored resize families. The three dependent slices
remain in progress until their conformance and measurement obligations finish.

## Same-target frozen reference

S27 benchmark preparation at `e395b16241f26443a43a049a0fbefb7166215fb8`
exposes one native-versus-Wasm byte difference in the 65x33 Oklab adaptive
blue-noise fixture. The native frozen output has red 95 at byte 6792. The
unchanged frozen Wasm output has red 94. The actual package matches all 8,580
frozen Wasm bytes.

Forward Oklab rounding explains the difference. The mask, threshold, neighboring
coordinates, and remaining output bytes agree. Preserve the original fixture
and diagnostic outputs under `v1-s27-bench/target/s27-preparation`.

S29 independently exposes seven native-versus-Wasm differences among 367
Yliluoma fixtures. A minimized frozen CIEDE2000 case returns native indices
`[2,2,1,1]` and Wasm indices `[2,2,1,2]`. Frozen CIELAB conversion already differs
before the mixture search. The S29 agent is checking the complete fixture set
against an independent frozen-only Wasm build.

The benchmark repair runs a frozen-only Wasm oracle in the measured browser,
outside measurement. Its separate context closes before package initialization,
warmup, or timers. Existing asset provenance binds its source, dependencies,
compiler, and emitted bytes. The oracle independently checks complete case
identity. Browser exactness uses that target-local result; native verification
stays native. Retain cross-target differences as separate diagnostics.

This changes benchmark tooling, not production arithmetic, frozen files,
tolerances, or expected bytes by hand. No measurement starts until the repaired
protocol passes its identity and exactness tests.

## Diffusion review

Root reviewed the ring and public ABI through `ffe996bf`. No actionable
correctness finding emerged. Diffusion uses three source-initialized f32 rows,
preserving each frozen f64 contribution and f32 store. Fixed-index pixels remain
sinks. Shared resize, color, palette, image, and spec trees match S26 exactly.
The quantizer adds read-only matcher/converter accessors.

The agent reports 360 frozen public vectors passing in Chromium, Firefox, and
WebKit. Native comparison scopes both borrow source and include preparation,
allocation, and destruction. The literal full-image subject remains the
accepted benchmark baseline. The ring candidate remains unselected.

## Next checkpoints

- Finish and independently review the target-local oracle protocol.
- Finish S28/S29 clean public and benchmark preparation checkpoints.
- Drain all implementation agents, builds, and tests before fresh measurements.
- Run only the declared serial trials. Retain noise or rejected candidates.
- Open stacked PRs and update the slice table after each delivery.

No PR merges, releases, publication, deployment, or rollout are authorized.
