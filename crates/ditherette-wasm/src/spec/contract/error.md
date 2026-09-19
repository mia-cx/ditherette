# Structured error spec

## Inputs and outputs

`error.rs` defines `ErrorCode` and `DitheretteError { code, path, message }`.
The constructor accepts a category, field path, and readable explanation. It adds no validation or allocation policy.
Serde encodes categories as kebab-case tags. Display prints the path followed by the message.

## Semantic rules

Callers recover through the stable code and path, not by parsing the message.
Request validation uses `InvalidRequest` for unsupported versions, `InvalidPalette` for empty palettes,
`InvalidImage` for invalid source storage or source-derived output sizes, and `InvalidSettings` for invalid settings.
Explicit output-size failures retain `InvalidSettings`, including when the source itself is valid.

The complete category set also includes `UnsupportedOperation`, `Capability`, `Initialization`,
`MemoryLimit`, `WasmMemoryUnavailable`, `Disposed`, `ReentrantCall`, `Callback`, and `Runtime`.
These names form a shared vocabulary; this module does not manufacture those failures.

## Correctness and production obligations

A failed call returns this diagnostic without a partial image.
Production must preserve the category and offending field path across its boundary adapters.
The readable message explains the failure without replacing those structured fields.
This module does not recover from errors, translate exceptions, or optimize string storage.
