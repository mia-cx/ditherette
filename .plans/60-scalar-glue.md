# S19 scalar glue factory subtask

Issue #60. Base `5e45b7b6677d1fe4241a8109c39818a36c44b61d`.
This child owns build generation, package staging, and focused isolation fixtures.
Rust baseline/processor work and the public nearest API remain with their assigned owners.

## TODOs

- [x] Generate a private scalar binding factory through a checked AST rewrite and prove closure/import isolation.
- [x] Wire scalar generation and declaration-first package staging, with focused layout/type-resolution checks.
- [x] Verify the factory against actual generated scalar glue without changing legacy artifacts; record validation and delivery.

Normal web glue and website URLs remain intact. Root imports remain inert.
No benchmarks, PR creation, merges, publishing, tags, or deployments run in this subtask.

The first three Node fixtures pass through `pnpm --filter ditherette-wasm test:glue`.
They cover independent concurrent initialization, mutable views/reference state, imported closures, exported classes,
shared stateless helpers, inert factory-module import, rejected export-shape changes, and preserved original glue.
The crate pins TypeScript 6.0.3 as its AST parser; the lock change only adds that existing dependency to its importer.

Both staging fixtures pass through `pnpm --filter ditherette test:staging`; package `check` also passes.
The real TypeScript compiler resolves a source import through staged declarations and preserves its emitted relative path.
The fixture imports that emitted file, checks sibling assets and threaded snippets, and rejects missing factory output.
Scalar builds generate the private siblings after wasm-pack; package builds stage before TypeScript compilation.

## Delivery validation

`pnpm --filter ditherette build` passes, including both pinned scalar/threaded builds, actual asset staging, and TypeScript emission.
The three generation tests, two built-glue tests, and two staging tests pass together. Package `check` passes.
The actual generated factory declaration passes strict NodeNext TypeScript checking. Targeted Prettier and `git diff --check` pass.
Importing the built package root with fetch and Wasm compilation/instantiation replaced by throwing functions succeeds without invoking them.

Built-glue fixtures instantiate one compiled module concurrently through two factories.
They prove separate memories, externref tables, exported closures, independent cached-view refresh after memory growth, and error-import recovery.
They also exercise offset byte views, a custom Response, and the sibling default Wasm URL.
These Node binding checks do not claim public Node support, browser conformance, or allocation safety of inherited methods.

The wrapper can lazily import `./wasm/scalar/ditherette_wasm.factory.js` and call `createScalarBindings()` once per processor.
Its returned namespace has the original generated declaration type. Initialize each namespace once through its `default` method.
Separate namespaces support concurrent initialization; repeated concurrent initialization of one namespace retains wasm-bindgen's existing behavior.
Static helper imports remain shared and must be stateless. The TypeScript parser is build-only.

Rust sources, Cargo files, the workspace policy, package source index, and website staging script are unchanged from the task base.
The existing root import remains empty and inert. Public wrapper and bounded Rust processor implementation are separate tasks.
