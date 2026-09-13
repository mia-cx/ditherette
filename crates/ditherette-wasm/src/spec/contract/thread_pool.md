# Per-instance thread-pool ownership

`ThreadPoolModel` adds resource ownership to the existing `lifecycle::initialize` selection rules.
It tracks worker-handle counts and the threaded artifact's shared-memory reference, without creating either resource.

| Transition | Required ownership result |
| --- | --- |
| Disabled or incapable Preferred selects scalar | Zero workers; no shared memory |
| Capable threaded attempt starts | Own the selected artifact's shared-memory reference; record each worker immediately |
| Threaded attempt fails | Release every partial worker and the shared-memory reference before fallback or error |
| Preferred fallback completes | Scalar execution, zero workers, no shared memory |
| Required attempt fails | Existing capability or initialization error; no scalar fallback |
| Threaded attempt succeeds | Keep its nonempty pool and shared-memory reference until teardown |
| Disposal or host termination | Release owned resources once; initialization cannot resume |

The model represents cleanup as a completed synchronous ownership transition.
Runtime code must finish actual cleanup before making the corresponding fallback transition.
Worker counts describe handles acquired so far, including workers whose startup handshake has not finished.
They do not introduce a public worker-count option, scheduler, or pool-sizing policy.

Package disposal first passes `InstanceModel`'s idle/reentry guard.
`host_terminated` instead describes the host destroying its processing worker and associated pool.
The host also discards that worker's caches and rejects stale progress/results.
Neither action adds cancellation to the five synchronous processing methods.
Dropping shared-memory ownership does not promise immediate garbage collection or Wasm page shrinking.

The requirements come from [decision 36](https://github.com/mia-cx/ditherette/issues/36#issuecomment-5455984344),
[decision 37](https://github.com/mia-cx/ditherette/issues/37#issuecomment-5562808049),
and [decision 24](https://github.com/mia-cx/ditherette/issues/24#issuecomment-5562864637).
