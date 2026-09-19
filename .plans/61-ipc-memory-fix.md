# S20 browser IPC allocation correction

## Cause

Trial one retained a 14,237,948-byte browser request before its Node process reached roughly 1.5 GiB RSS.
The coordinator recorded 61 successful results and one failed worker. It retained all artifacts and confirmed child cleanup.
See the coordinator-owned `61-trial-01-failure.md` for the complete trial record.

Playwright 1.59.1 `serializeArgument` converts every numeric array element into a separate `{ n: value }` object.
Its local launchServer/connect path then serializes and parses that expanded object graph again.
The transport passed complete source/reference arrays through `page.evaluate` and returned complete output arrays through the same RPC.

An untimed diagnostic isolates the first expansion. No browser or image-processing call is necessary to reproduce it.
The retained request exhausts a 128 MiB Node heap twice during `serializeArgument`, exiting 134.
The same request passes ordinary JSON parsing/stringification under that heap limit, with peak RSS 144,628 KiB.
Core dumps were disabled for both failure reproductions. These are Node heap-limit failures, not another host or browser OOM.

## Correction

Implementation commit `38689b5` changes only browser transport and its focused fixtures.
The Rust schema, reference, image kernels, and public-call timers remain unchanged.

1. Serve the identity-bound request JSON from a one-use loopback data route.
2. Let the page post its result JSON to a separate one-use route.
3. Pass only route URLs, the page module path, and a small acknowledgement through Playwright.

The data routes are separate from executable assets. Their reserved prefix cannot collide with the asset manifest.
Static imports still require manifest entries. Other network traffic remains blocked.
The data-route exclusion uses a server-side Playwright regular expression. Intercepting the POST would otherwise copy its body into RPC again.

Input is an already-serialized, finite request served once. Result uploads have a request-derived byte bound.
The bound covers worst-case RGBA8 JSON, the configured sample count, and fixed metadata allowance.
Both declared Content-Length and actual streamed bytes are checked. Oversized chunked bodies fail without retaining excess data.
Duplicate requests/submissions, wrong methods/content types/origins, malformed JSON, and undeclared requests remain failure evidence.
The trial cannot succeed by ignoring a rejected HTTP status.

Renderer crash, page closure, page error, and browser disconnection reject pending control work.
The observer removes its listeners after success or failure. No artificial timeout creates replacement samples.
Existing owned-browser/server cleanup still runs afterward.

## Untimed validation

The large echo uses the retained request's complete source and reference arrays.
It returns both arrays to exercise a larger response than the real resize result.
Only its declared output-height bound is enlarged to accommodate this diagnostic echo.
This is not a processing request or performance sample. No image kernel or timing loop runs.

With Node `--max-old-space-size=128`:

| Observation | Result |
|---|---:|
| HTTP request JSON | 14,237,950 bytes |
| HTTP result JSON | 14,039,177 bytes |
| Source channels checked byte-for-byte | 3,145,728 |
| Reference channels checked byte-for-byte | 786,432 |
| Largest observed Playwright RPC message, either direction | 1,240 bytes |
| Peak Node RSS, including final result JSON serialization | 271,940 KiB |

The 128 MiB bound applies to V8's heap, not total process RSS.
The fixture's parent owns the Chromium server, so a bounded child failure cannot orphan it.

Focused validation passes 17 tests. Coverage includes the large echo, an actual Chromium renderer crash, listener cleanup, oversized chunked POST, duplicate submissions, reserved-route collisions, preflight mismatch retention, and timing-core call counts.
Installed-tarball conformance also passes Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, with four reported tests.
Those three engines now check untimed HTTP exchange alongside the existing public-package and real-TypeScript checks.
The inherited TypeScript 2→49 mismatch remains visible and unchanged.
Prettier and `git diff --check` pass for the changed files.

### Commands

```sh
ulimit -c 0
node --max-old-space-size=128 scripts/benchmark-public-ipc-fixture.mjs rpc RETAINED_BROWSER_REQUEST_JSON
node --max-old-space-size=128 scripts/benchmark-public-ipc-fixture.mjs json RETAINED_BROWSER_REQUEST_JSON
DITHERETTE_BENCH_IPC_REQUEST=RETAINED_BROWSER_REQUEST_JSON node --test scripts/benchmark-public-browser.test.mjs scripts/benchmark-public-timing.test.mjs scripts/prepare-benchmark-typescript.test.mjs scripts/benchmark-public-ipc.test.mjs
```

The first command intentionally reproduces heap exhaustion. Do not run it during a measurement lease.
Omit `DITHERETTE_BENCH_IPC_REQUEST` for the permanent deterministic large-array fixture.
The retained input was `target/s20-public-trial-01/chromium-results/pair-003-case-003-accepted.request.browser-request.json` under the coordinator's benchmark crate.

No benchmark ran during this correction. Every owned browser, HTTP server, test child, and diagnostic process exited.
The failed trial remains invalid evidence. The coordinator must rebuild and prepare fresh artifacts before any new measurement.
