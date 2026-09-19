# Prepared public browser trials

The browser worker extends [paired execution](PAIRED.md). It never starts another benchmark worker.
Its only child is Node, which owns the browser and local asset server through the existing lease transport.
All preparation commands read built artifacts. They do not run timed image workloads.

## Build and snapshot

The crate-owned `scripts/prepare-public-benchmark.mjs` builder emits `bundle-source.json` and `build-provenance.json`.
The Rust `BundleSource` accepts package, compiled TypeScript, scripts, source_checkout, provenance, and explicit entry paths.
Every source/output file has a sorted relative path, length, and full SHA-256 digest.
The source revision must match the clean checkout, every committed source blob, and the role's worker revision.
Git assume-unchanged and skip-worktree flags cannot hide changed inputs.
Tool records bind names, versions, and complete executable/compiler digests.

`BrowserSources` contains accepted/candidate BundleSource values and one RuntimeSource.
RuntimeSource specifies engine, Node path/version, browser path/version, complete browser_assets root,
Playwright and playwright-core package roots, launch_args, headless, and cross_origin_isolated.
Node versions retain their `v` prefix. Browser versions match `browser.version()` exactly.

The preparer copies complete package, TypeScript, scripts, browser, and Playwright trees.
It preserves internal file-link content as regular files and rejects escaping or directory links.
Snapshot files lose write permissions. Validation detects added/missing files, writable replacements, and byte changes.
Content identities exclude storage roots but include entrypoints, provenance, private libraries, and runtime options.
System libraries remain part of the recorded host trust boundary, not an invented full OS snapshot.

For task-local WebKit dependencies, stage a relocatable launcher first:

```text
ditherette-bench-pair prepare-webkit INSTALL_ROOT PRIVATE_LIBRARY_DIRECTORY NEW_DIRECTORY
```

Use the resulting directory as browser_assets and its `webkit` file as browser.
The reviewed launcher resolves its executable and libraries from its own directory.
Other script launchers fail validation; copying a launcher with absolute external paths is insufficient.

```text
ditherette-bench-pair check-browser-source BUNDLE_SOURCE_JSON FULL_SOURCE_REVISION
ditherette-bench-pair prepare-browser EXPERIMENT ACCEPTED_WORKER FULL_REV CANDIDATE_WORKER FULL_REV SOURCES_JSON NEW_DIRECTORY
```

Preparation writes assets under `NEW_DIRECTORY/assets` and paired worker evidence under `NEW_DIRECTORY/pair`.
Existing destinations are never overwritten. Failed preparation retains partial evidence for inspection.

## Owned trial and exact proof

The coordinator invokes `paired-browser-trial` on one clean, hashed worker.
The worker recomputes the frozen nearest output using the typed anchor before starting Node.
It writes a separate `*.browser-request.json` containing the reference output.
Node receives that absolute path as its only file argument and emits one bounded JSON line.
The worker retains raw stdout, then checks all input/settings/role/runtime identities and sample fields.
The coordinator also checks complete asset trees before and after each worker.

The page checks an untimed public call against the supplied frozen bytes before warmup or timing.
A mismatch returns actual output with `timing_skipped: "reference-mismatch"` and zero work counts.
The worker writes `*.preflight-mismatch` S05 records/images and rejects the trial without samples.
Ordinary measured byte differences remain S05 conformance failures with retained review artifacts.

Zero timer samples remain raw evidence. Native comparisons, public latency, throughput, and initialization stay separate.
S19 has no application cache or content hashing. Fresh/primed instances cannot claim cold/warm application-cache behavior.
Later slices extend the typed operation registry rather than borrowing unrelated labels.
