# S41 disk recovery

The release run exhausts disk space. On resumption, no owned benchmark, browser,
compiler, or build process remains. The release completion file is zero bytes;
it is not evidence of successful completion. Preserve the interrupted attempt.

Root removes about 72 GiB of finished-worktree target data, restoring 76 GiB of
available space. Source worktrees, branches, and PRs remain unchanged.
Compact reports, event journals, provenance, small logs, and package tarballs are
gzip-archived under root `benchmark-results/retained-reports-2026-09-09`.
Each archived report keeps its original relative path and verified SHA-256.
The archive totals 38 MiB. Old compiler output and browser copies are rebuildable;
discarded full raw request/result payloads are no longer retained.

This supersedes earlier instructions to keep every immutable trial snapshot.
Finished worktrees clear all target trees, not only Cargo compiler profiles.
The active coordinator and current S41 artifact targets remain until their work ends.
The remaining runner checks for 12 GiB free space before every case and stops
launching when that reserve is unavailable. It still waits for each owned child.

The interrupted release lane has complete reports for all 15 Chromium cases and
the first nine Firefox cases. Firefox random perturb lacks a complete report.
All remaining Firefox and WebKit release cases are incomplete or unrun.
Completed reports remain valid for their recorded artifacts; no partial worker
is combined with a rerun or presented as a completed pair.
