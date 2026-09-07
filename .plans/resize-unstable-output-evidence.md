# Retain unstable diagnostic outputs

Base `60e52acbede63c1aa4df7999e18dd8be8fc5e0b3`.
An opted-in nonexact diagnostic previously threw when its final output differed from preflight.
That discarded both actual images before the worker could preserve review artifacts.

## TODOs

- [x] Return the preflight actual image as an explicit instability marker beside the final actual output.
- [x] Preserve both review bundles and reject normal trial publication; validate focused fake protocol fixtures.

## Contract

`BrowserTransportResult.unstable_output` is optional and contains the preflight actual image.
The existing `output` field retains the final actual image. Presence of the new field marks instability.
The worker accepts this marker only with explicit `measure_nonexact`, real changed bytes, and a preflight reference mismatch.
It retains raw JSON and separate `preflight/` and `final/` S05 bundles, then returns an error.
No `TrialResult` is published. Returning to reference-exact final bytes does not erase the earlier failure.
Without opt-in, the original mismatch response still reports zero timing work.
Stable nonexact outputs still use the existing Incorrect gate; this change grants no conformance or release approval.

HTTP and worker response bounds allow two complete images only for opted-in diagnostic requests.
Production, frozen reference, and public package files remain unchanged.

## Validation

- `node --test scripts/benchmark-public-browser.test.mjs`: nine passing tests.
  The changing-output fixture uses a fake clock and fake adapter, not image-processing measurements.
  A 320-kilobyte HTTP body proves both actual arrays fit beyond the previous single-output limit.
- `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --test browser_worker --test paired_browser`: four worker and eleven paired-protocol tests pass.
  Fixtures verify retained JSON/PNG bytes, strict opt-in, rejection before publication, and unchanged no-timing mismatch behavior.

The first new HTTP fixture lacked the required JSON content type, then expected 200 instead of the existing 204 response.
Those fixture assumptions were corrected; the server contract did not change.
No real benchmark or browser measurement ran.
