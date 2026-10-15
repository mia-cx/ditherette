# #412 Reuse IndexedDB connections with lifecycle recovery

## Summary

Replace per-operation connection setup with lazy shared ownership. Keep version 1,
the records store, source/processed keys, and transaction-completion guarantees.

## Acceptance criteria

- [x] Serial and concurrent operations coalesce opening and reuse one connection.
- [x] Close, version changes, rejected opens, and abnormal closure release ownership.
- [x] Aborted/failed transactions reject without write retries; request success cannot resolve a write.
- [x] Conditional source rollback and source/output transaction ordering remain intact.
- [ ] Isolated Chromium tests, caller tests, static checks, and the web build pass.

## TODOs

- [x] Implement shared ownership and lifecycle recovery with focused browser coverage.
- [~] Run final caller/static/build checks and record the verified handoff.

## Notes

- Supplied clean branch advanced from f420d494 to exact base 3e05dac9 by an empty rebase.
- Own db.ts, focused database tests, this plan, and the changeset. Callers remain unchanged.
- Completion means a non-draft PR closing only #412, with exact head and results handed to the parent.
- Evidence lives under `/home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/indexeddb-connection/`.
- [IndexedDB connection close algorithm](https://w3c.github.io/IndexedDB/#close-a-database-connection)
  requires existing transactions to finish on explicit close. Only abnormal closure fires `close`.
- [IndexedDB transaction creation](https://w3c.github.io/IndexedDB/#dom-idbdatabase-transaction)
  throws `InvalidStateError` for a closing connection. Propagate this operation's failure and
  invalidate ownership for the next operation, without replaying any write.
- [IndexedDB version changes](https://w3c.github.io/IndexedDB/#eventdef-idbdatabase-versionchange)
  require closing the old connection to allow another context's upgrade/deletion.
- Source generation checks, transactional conditional rollback, and the client's persistence
  timer cancellation remain at their existing caller boundaries. No new operation queue or retry.
- Deterministic setup counts qualify this slice. Timed save/restore work and host policy stay deferred.
- Chromium correctness run passes all 31 database tests. A temporary copy of the exact base
  implementation passes the same two setup-count sequences with eight opens/connections each.
  This implementation uses one open/connection for both eight-operation sequences.
- Combined baseline/current run: `pnpm exec vitest run --project client
  src/lib/processing/db.browser.spec.ts src/lib/processing/db-setup-baseline.browser.spec.ts`,
  exit 0, 33 tests. Baseline fixtures now live only in the evidence directory.
- Real browser coverage includes upgrade/deletion version changes, aborted upgrades, synchronous
  open failures, closed-connection failures, three write paths aborted after request success,
  native request errors, source identity fields, and old/new source/output operation ordering.
- Abnormal-close notification is simulated on a real closed connection because IndexedDB has no
  public force-close method. Explicit close, version changes, transactions, and data use real IndexedDB.
