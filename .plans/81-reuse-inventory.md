# S39 website scheduling reuse

Start only from an explicit join of validated S34 and S38 PR120 at `0305456bc25259a92d46ded245ae09aaf407be07`.
Read S39, the current execution contract, and issue 39's settled rollout resolution.
This slice uses existing status/error presentation. It does not redesign the UI or activate the development flag.

`processing/client.ts` already owns a singleton processing worker, request IDs, active rejection, source loading, and a 180 ms slider debounce.
Explicit cancel terminates the worker, clears its source identity, and invalidates the request ID.
Settings supersession currently posts a cancel message instead. Synchronous Wasm cannot receive that message during processing.
Reuse the explicit termination path when debounced replacement starts. Invalidate stale results and progress immediately when settings change.
Keep the last valid preview under the existing source/crop rules. Preserve the existing persistence and metrics responsibilities.
Guard stale worker callbacks before any shared progress/error state mutation, including malformed messages and worker errors.

`processor.worker.ts` already serializes asynchronous pipeline requests and forwards progress with a request ID.
`ProcessorWorkerPipeline` lazily initializes one package processor per worker and translates settings through `package-adapter.ts`.
S38 does not pass a progress callback and provides no automatic fallback. S39 connects the now-available public progress contract.
The adapter owns crop packing, palette order, alpha settings, output metadata, and current website strength scaling. Reuse it unchanged where possible.

Fallback belongs to the page session, not only the replaceable worker. Worker replacement must not retry a known failed initialization repeatedly.
Catch load, initialization, and capability failures at that boundary. Processing, invalid-input, memory-limit, and callback failures remain visible.
Verify a request's faithful TypeScript support before fallback. Similar algorithm names do not establish equivalent semantics.
Existing evidence records TypeScript area and reduction-bilinear semantic differences; preserve those findings when defining support.
Report activation through existing status/error/warning mechanisms. No user-facing backend selector is added.

Reuse S34's actual nested-worker lifecycle observer for host termination where practical.
Focused fixtures cover replacement after debounce, immediate stale rejection, last-preview retention, page-session fallback, and visible runtime failures.
Do not add another cancellation API to the five synchronous package methods.
