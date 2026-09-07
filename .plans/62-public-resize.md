# S21 public area and bilinear integration

Base `467542f49ce3f600e5b03aeef574a97be554ae15` restores landed production kernels and shared helpers.
Historical literal-copy records describe a superseded decision. They do not direct this implementation.

## TODOs

- [x] Add reusable fallible capacity reservations for nested plans and scratch. Two focused allocation tests pass.
- [ ] Integrate fallible preparation and caller-owned scratch into landed area/bilinear paths.
- [ ] Expose canonical area/bilinear requests through the bounded Rust processor and private/public package bindings.
- [ ] Verify original outputs, frozen-reference differences, allocation failures, and public ownership; push the implementation checkpoint.

## Boundaries

Keep all landed pixel arithmetic, support, ordering, and fast-path dispatch.
Frozen area has no anchor; frozen bilinear has an anchor but no configurable support policy.
The public processor owns transient plans and scratch. Its capacity budget excludes returned JS-owned bytes per resolution #37.
S22 consumes the shared allocation helper, then extends the public enum after stacking on this slice.
The coordinator owns benchmark registration, measurements, progress, and PR filing. No measurements run during this implementation.
