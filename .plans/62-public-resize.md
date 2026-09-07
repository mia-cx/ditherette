# S21 public area and bilinear integration

Base `467542f49ce3f600e5b03aeef574a97be554ae15` restores landed production kernels and shared helpers.
Historical literal-copy records describe a superseded decision. They do not direct this implementation.

## TODOs

- [x] Add reusable fallible capacity reservations for nested plans and scratch. Two focused allocation tests pass.
- [x] Integrate fallible preparation and caller-owned scratch into landed area/bilinear paths. Six focused tests pass.
- [x] Expose canonical area/bilinear requests through the bounded Rust processor and private/public package bindings.
- [x] Verify original outputs, frozen-reference differences, allocation failures, and public ownership; push the implementation checkpoint.

## Boundaries

Keep all landed pixel arithmetic, support, ordering, and fast-path dispatch.
Frozen area has no anchor; frozen bilinear has an anchor but no configurable support policy.
The public processor owns transient plans and scratch. Its capacity budget excludes returned JS-owned bytes per resolution #37.
S22 consumes the shared allocation helper, then extends the public enum after stacking on this slice.
The coordinator owns benchmark registration, measurements, progress, and PR filing. No measurements run during this implementation.

Fallible constructors keep the landed nested metadata layouts. Their iterators share the original tap/overlap formulas.
The public scratch entrypoints reuse the original arithmetic loops. Legacy convenience calls keep their allocation ownership.
Exact area paths need no plan buffers. Bilinear needs one f32 source-width row when height changes.

## Public integration

`prod/pipeline/resize.rs` owns private prepared-plan dispatch. The processor accounts for its enum record, nested metadata, scratch, and image capacities.
Private ABI is `privateResize(input, sourceWidth, sourceHeight, outputWidth, outputHeight, algorithm, anchor, resultSink) -> u32`.
Algorithm codes are 0 nearest, 1 area, and 2 bilinear. Area passes unused anchor 0. Existing error codes and paths stay unchanged.
`ResizeRequest` replaces the private `NearestRequest` name; all callers use the new name without an alias.
S22 adds its variants and support discriminator after stacking on this interface.

## Validation

Implementation commit `35169fc0059d53f3166654e7e9a549353f0ac489` is pushed on `impl/v1-s21-area-bilinear`.
All 287 native tests pass, summed from the actual test-result groups. The restored base remains in ancestry.
The scalar and pinned threaded package builds pass. The shared budget helper uses the older compiler's supported Option API.
Fourteen package interface tests, six private ABI tests, and installed-tarball checks in all three engines pass.
Browsers are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4. The browser test reports four passing tests including its parent.
The native public matrix covers ten shapes, all nine anchors, alpha extremes, hidden RGB, and maximum source/output side lengths.
Maximum squared RGBA distances from the frozen reference are 1 for area and 3 for bilinear, within the landed distance bound 2.
Actual allocation failure injection covers each nested plan, scratch, source, and output reservation, with zero retained bytes and recovery.
Exact-capacity requests pass; one byte less rejects before allocation. Caught boundary failures release transient preparation and preserve previous outputs.
The separate trusted S18 guard and Rust formatting pass. Frozen spec, image infrastructure, and policy remain unchanged.

Native and complete public-call measurements remain coordinator-owned acceptance work. No benchmark or optimization experiment ran here.
