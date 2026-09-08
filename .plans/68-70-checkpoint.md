# S27 through S29 integration checkpoint

S27, S28, and S29 are delivered in open, unmerged PRs #116, #118, and #117.
Coordinator `246297c9` contains accepted S26, all restored resize families,
both benchmark snapshot fixes, and delivered S27. S30 now joins the two siblings.
The stack ledger records exact delivered heads and selected implementations.

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
frozen-only Wasm build. Both final S29 roles pass that complete coverage through
the permanent identified oracle. All native/Wasm differences remain recorded.

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
timers. Follow-up `f8a2cc11` rejects shared backing stores before retention,
because structured cloning preserves SharedArrayBuffer sharing. All 30 relevant
browser, timing, and Chromium IPC tests pass. Three review threads are resolved. Palette
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
accepted benchmark baseline. Fresh measured source `058f276d` includes the
permanent oracle and current snapshot fixes without changing the ring.
Its 128-worker trial retains 2,356 exact samples. All eight native cases pass,
with 8–91% lower medians. Root selects the bounded ring implementation.
Chromium and Firefox self-pairs pass. WebKit's Floyd-Steinberg sRGB-byte control
has pair noise, and its Atkinson sRGB-byte control is resolution-limited.
Those self-pair gaps remain S41 release work, not passing public speedup evidence.
The complete record is `.plans/69-measurement.md` on PR #118.

## Yliluoma candidate

Literal baseline `50cd96d1` and converter candidate `fdb3921a` pass the 367-case
three-browser suite. The only candidate behavior change reuses the existing
prepared color converter. Pair search, ratios, ties, placement, and alpha stay
unchanged. The completed trial retains 128 reaped workers and 2,560 exact samples.
All four runtime gates remain inconclusive from paired timing noise.
Root follows the declared selection rule and retains the literal implementation
in PR #117. The candidate and all observed gains remain separate for S41.

Both branches include the indexed Yliluoma worker classification fix. The
previous JavaScript fixture checks did not exercise that Rust rejection path.
Its focused regression fails before the fix and all seven worker tests pass
afterward. No actual measurements used the incorrect classifier.

## Next checkpoints

- Validate S30's prerequisite join from all delivered heads before runtime edits.
- Rebuild the combined package. Coordinator field tests still load S26 Wasm with
  SHA-256 `6c000fa7691f3aa2e6aa36537983d688aafd5c0eb726fa3bf2d5f1d159ffee0f`,
  identical to its retained S26 tarball. Three blue-noise field checks therefore
  fail as unsupported; TypeScript and 28 protocol checks pass.
- Implement only the missing full process composition, with existing kernels,
  complete memory preflight, recipe paths, and exact RGBA8 boundaries.
- Preserve reports and snapshots after compiler-output cleanup. S30 alone owns
  the active S24 quantize cache; returned S22/S23/S24-bench profiles are removed.

No PR merges, releases, publication, deployment, or rollout are authorized.
