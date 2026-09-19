# S24 review follow-up

PR #113 review comment 3952910568 exposes mutable benchmark evidence.
The collector holds the first public result directly. A later call can mutate
it and return fresh storage, hiding changed output or rewriting saved evidence.

Baseline browser/timing protocol tests pass 26 cases. The new regression fails
because later mutations rewrite the first/distinct bytes. The fix takes private
typed snapshots outside timers and checks retained-first mutation before the
next output. Identity checks still use the original objects and buffers.
The 64 MiB budget now accounts for both originals and both snapshots.
All 27 browser/timing tests pass after the fix. Production and frozen code stay
unchanged. Fresh benchmark artifacts must include the corrected collector.
Older immutable trials remain historical evidence, not reruns of this protocol.
The two actual Chromium IPC checks also pass. An expanded direct Node invocation
included three legacy transport tests requiring a Rust-owned lease descriptor;
they reject that invocation before their fixture callbacks. Their source is
unchanged. The corrected relevant suite excludes those lease-owned tests.

Comment 3952910560 requests a fixed palette-validation work limit. Oversized
palettes remain valid under decision 40 and frozen request validation. The
public boundary documents full tail validation and tests malformed entry 257.
Full validation requires linear reads. A fixed limit rejects currently valid
palettes; prefix-only validation accepts currently invalid tails. The current
ABI already retains at most 257 codes. No repository caller generates huge
palettes; website creation/import limit them to 256 entries. Host JavaScript
getters and proxies can execute arbitrary caller code regardless of a count cap.
Keep the approved contract. A practical input cap needs a separate decision.

Follow-up comment 3955731533 identifies SharedArrayBuffer-backed result views.
Structured cloning preserves their shared backing, so they cannot form private
snapshots. The collector now rejects shared result storage before retaining it.
This applies to RGBA bytes, indexed bytes, and palette bytes, including threaded
subjects, whose public results still need exclusively owned durable storage.
The new test fails on the previous collector and passes after the guard.
All 30 relevant browser, timing, and Chromium IPC checks pass. Pending role
artifacts remain unmeasured and must rebuild with this final collector guard.
