# S41 maximum-output resource check

The single approved untimed Chromium probe fails in Playwright's pipe transport.
The capped lane remains incomplete. No performance samples or full-size success
claim result from this attempt. Do not launch the capped measurement matrix.

The retained probe uses selected source `a895267baea624a6e89bfcef6c5147f170e8a8f7`
and the ordinary package from the fresh current preparation. Its fixed recipe is
nearest Process from 1×1 to 8192×8192 indexed pixels, with explicit 384 MiB
benchmark retention. Package memory limits and frozen semantics remain unchanged.

Node 24.19.0 exits 1 with `ERR_STRING_TOO_LONG` in Playwright's
`PipeTransport._dispatch`, while converting a received buffer to a string.
The message reports a maximum of `0x1fffffe8` characters. The uncaught transport
error prevents the probe's final report. No stage completion is inferred from
the missing report, and this does not prove a production allocation failure.

The wait wrapper records `child_peak_rss_kib: 2027572`, about 1.93 GiB.
This is the maximum individual child-process RSS, not concurrent process-tree
memory. After exit, a read-only process check finds no owned Chromium, probe,
or benchmark process. The retained evidence includes the exception and wait status.

Files remain under `.worktrees/v1-resize-integration/target`:

- `s41-capped-probe-01.log` and `s41-capped-probe-01-rss.log` retain the failure.
- `s41-capped-probe-01/` retains the incomplete probe directory.
- `s41-capped-probe-notes.md` and the probe scripts record intended checks and limits.
- `s41-capped-probe-plan.json` retains the exact input and recipe.

The probe targets full frozen-oracle, Process/staged equality, and JS transport.
It does not exercise Rust's final full-size decoder or the native full-size oracle.
Tiny codec tests remain valid but do not establish maximum-size feasibility.
There is no retry, substituted smaller case, limit increase, or release-gate waiver.
