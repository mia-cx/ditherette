# S27 through S29 integration checkpoint

S26 PR #115 is open and unmerged. Coordinator `6d1e3827` contains its accepted
implementation, all restored resize families, and the benchmark snapshot fix.
S25 and S26 carry that fix at `ff2d1232` and `68f058e9`. The three dependent slices
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
before the mixture search. All 367 fixtures pass against an independent
frozen-only Wasm build. The permanent identified oracle now needs that same
full fixture coverage after the protocol join.

The benchmark repair runs a frozen-only Wasm oracle in the measured browser,
outside measurement. Its separate context closes before package initialization,
warmup, or timers. Existing asset provenance binds its source, dependencies,
compiler, and emitted bytes. The oracle independently checks complete case
identity. Browser exactness uses that target-local result; native verification
stays native. Retain cross-target differences as separate diagnostics.

The oracle protocol is complete at `6673de9c`. It passes 32 focused Rust tests,
29 Node tests, and 63 fixtures in each browser. Frozen content, compiler,
dependency, and syntax checks pass. This changes benchmark tooling, not
production arithmetic, frozen files, tolerances, or expected bytes by hand.

PR #113 snapshot fix `eb1725ef` keeps private first/distinct evidence outside
timers, so later producer calls cannot rewrite it. All 29 relevant browser,
timing, and Chromium IPC tests pass. Both review threads are resolved. Palette
validation keeps its approved oversized-input behavior and documented tail
checks. Earlier trial records remain historical, not reruns of this protocol.

## Diffusion review

Root reviewed the ring and public ABI through `ffe996bf`. No actionable
correctness finding emerged. Diffusion uses three source-initialized f32 rows,
preserving each frozen f64 contribution and f32 store. Fixed-index pixels remain
sinks. Shared resize, color, palette, image, and spec trees match S26 exactly.
The quantizer adds read-only matcher/converter accessors.

Checkpoint `8df7b483` passes 360 frozen public vectors in Chromium, Firefox, and
WebKit. Native comparison scopes both borrow source and include preparation,
allocation, and destruction. The literal full-image subject remains the
accepted benchmark baseline. The ring candidate remains unselected.

## Yliluoma candidate

Literal baseline `092f2dd0` and converter candidate `3b11261f` pass the 367-case
three-browser suite. The only candidate behavior change reuses the existing
prepared color converter. Pair search, ratios, ties, placement, and alpha stay
unchanged. The candidate remains unselected until fresh comparisons finish.

Both branches include the indexed Yliluoma worker classification fix. The
previous JavaScript fixture checks did not exercise that Rust rejection path.
Its focused regression fails before the fix and all seven worker tests pass
afterward. No actual measurements used the incorrect classifier.

## Next checkpoints

- Join the oracle and snapshot fixes into S27/S28/S29 without changing kernels.
- Validate the full S28/S29 matrices through the permanent identified oracle.
- Rebuild fresh native/public role artifacts and immutable comparison snapshots.
- Drain all implementation agents, builds, and tests before fresh measurements.
- Run only the declared serial trials. Retain noise or rejected candidates.
- Open stacked PRs and update the slice table after each delivery.

No PR merges, releases, publication, deployment, or rollout are authorized.
