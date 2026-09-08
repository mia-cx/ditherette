# S30 review corrections

Issue #71, PR #119. No new performance trial ran.

## Process memory accounting

Commit `aa4f78d1d89b43e35e9a390b4102693c7519a535` charges the temporary field converter only for separable dithering.
Other Process recipes never construct that converter. The previous charge rejected valid tight-budget calls.
The focused regression fails before the change and passes afterward. All five native Process tests pass.
The frozen-reference CI check passes at this commit, including its native and Wasm validation.
No image-processing kernel, shared resize helper, frozen reference, or validation rule changes.

The measured source remains `e5aae7bf0e1761af2f970b6da75d34cf3a813323`.
Its retained tarball does not validate the later memory-accounting change.

## Native attribution preservation

Commit `86a98935e9500670fafca8191364a436f3934379` clones nested attribution before replacing references with target-local Wasm outputs.
Previously, Firefox inherited Chromium's probe reference and WebKit inherited Firefox's reference under the `native_reference` label.
The new fixture-digest assertion fails on all three engines before the fix.
Afterward, all 431 main references, two area probes, and 16 Process compositions pass per engine.
An independent comparison confirms both retained native probes equal their untouched original native fixtures on all three engines.

This refresh uses the original measured package and oracle. It makes no new performance or allocation-limit claim.
The inherited area drift and target-local blue-noise diagnostic remain unchanged.
The original Firefox/WebKit provenance claims in the historical JSON summary are superseded by this refresh.
Original measurements, reports, and artifact hashes remain intact.

Refreshed evidence lives under `.worktrees/v1-s30-process/target/s30-review-conformance/`.

| Report | SHA-256 |
| --- | --- |
| `chromium-references.json` | `bfeccffc16f9a0873bfa9313cd5e664b349b50f68c7039eb35cec23389f5928d` |
| `firefox-references.json` | `6a4f9bf2ea3f607fb9830074dc090727a65f73ffe0690a47cc71441f50bc5c4f` |
| `webkit-references.json` | `44d3f50177dc73dab6559931bf6365386f4def3a0bbbd2f9b3b4d9058f67aa92` |
| Each engine's `*-process-composition.json` | `5c91ca00ea5227b7225d78cb7bbb09b7dfa1efada228d6b9c7748d2a94addeb6` |

The tarball SHA-256 remains `379c733b02bc67a24500d3ae825901d17d5fa342f93d114c20761da1aa9193b2`.
The oracle Wasm SHA-256 remains `300f61644c4b7757d1ad80b97c515121a5ad241fa0051e9827e448f0067ffb64`.
The original Process fixture SHA-256 remains `63ac50059dfe6fd4681d9afdbe9a1dca8394c19ae561b0d0bf2e3a3d986b6488`.
