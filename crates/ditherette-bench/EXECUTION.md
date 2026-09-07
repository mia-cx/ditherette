# Exclusive benchmark execution

Measurements require a quiet phase and the shared OS lease. The Rust CLI,
Criterion entry point, and browser transport use the same lease across worktrees.
Measurement support currently requires Unix. Other platforms return an explicit
unsupported-platform error before running a workload.

## Coordinator protocol

1. Build accepted and candidate executables and browser assets during implementation.
2. Stop dispatching agents. Wait for every implementation agent, compiler, build,
   and test command to exit. Record unrelated host load without stopping it.
3. Enter the quiet phase. Start the prebuilt executable through the lease helper:

   ```sh
   crates/ditherette-bench/target/release/ditherette-bench-lease --quiet -- \
     crates/ditherette-bench/target/release/ditherette-bench wasm-resize run nearest-thread
   ```

4. Wait for the helper to exit. Its child waits for the owned Node transport;
   Node waits for browser shutdown, then closes its fixture server.
5. Record the exit status and artifacts. Resume implementation only after the
   benchmark and its owned children exit. Failed runs are incomplete evidence.

`--quiet` attests that the coordinator completed step 2. The helper passes
`DITHERETTE_BENCH_QUIET=1` to its child. Direct CLI and Criterion invocations
require this same attestation before measurements. Help and registry queries
still acquire the lease but do not require a quiet-phase attestation.

The lock does not detect agents, compilers, or unrelated host activity. Quietness
remains the coordinator's responsibility. Build commands, including `cargo run`
and `cargo bench`, belong before the measurement phase; execute their prebuilt
binaries during it. Node is a transport entry point and rejects unguarded use.

## Lease ownership

`/tmp/ditherette-bench.lock` is the fixed host lease. It ignores the working
directory, repository revision, and `TMPDIR`. Unix `flock` owns the open file
description. Process exit closes its descriptors. The persistent file itself
is not evidence of a live owner and must never be deleted to acquire the lease.

Contention fails immediately with the lock path and a clear diagnostic. Wait
for the owner to finish. Permission failures also fail closed; there is no
worktree-local fallback or timeout that steals another process's lease.

`DITHERETTE_BENCH_LEASE_FD` identifies an inherited descriptor. The Rust guard
checks its inode and reasserts the OS lock. Each benchmark also acquires
`/tmp/ditherette-bench.execution.lock` through a fresh descriptor. Sharing the
outer lease therefore cannot authorize two benchmark executions or a nested
benchmark process.

The owner retains the lease while cleaning up a failed transport. SIGINT,
SIGTERM, and SIGHUP stop the owned child and wait for its cleanup before exit.
The Node transport also watches its parent's pipe for unexpected parent exit.
Playwright's own signal exits are disabled so transport cleanup owns shutdown.
Cleanup waits for the owned process; it does not kill unrelated processes or
expire a lease because shutdown takes longer than expected.

## Fresh cross-revision pairs

The [fresh-pair coordinator](PAIRED.md) uses the public
`ditherette_bench::lease` module from a separate executable:

```rust,ignore
let lease = Lease::exclusive()?;
for command in alternating_prebuilt_commands {
    let mut child = lease.spawn(command)?;
    let status = child.wait()?;
    // Stop on failure and retain the corresponding artifact evidence.
}
```

Set `DITHERETTE_BENCH_QUIET=1` on each `Command` after completing the quiet phase.
Bind commands to prepared executable identities. `spawn` consumes
the command and forwards the inherited lease descriptor. Keep the outer lease
until all trials exit. The execution guard remains local to each benchmark.

The helper accepts one direct executable. A fresh-pair coordinator owns the
lease itself and launches the accepted and candidate executables sequentially.
Do not start a coordinator or another benchmark from `ditherette-bench`.

## Controlled verification

Run these during implementation, with no measurement active:

```sh
cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test lease
cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --benches
```

The fixtures test lock contention across working directories, inherited execution
rejection, sequential ownership, stale completed handles, failed spawn, malformed
JSONL, signal cleanup, and browser startup/runtime failure. Browser fixtures own
a detached Node process in place of Chromium. They collect no benchmark timings.
The ignored Rust test is the fixture executable invoked by the lifecycle test.
