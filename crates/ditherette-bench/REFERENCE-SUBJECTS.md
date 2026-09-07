# Registered reference subjects

`ditherette_wasm::bench_subjects()` is the shared discovery registry. It contains
legacy resize timing subjects and callable `BenchSubject::Conformance` entries.
`list-subjects` and `describe-subject` expose both. Legacy resize measurement
selection skips conformance entries and rejects their IDs as timing subjects.

The concrete callable type lives in `bench_subjects::reference`, outside spec.
`ReferenceRequest::Processing` borrows the existing public `Request` enum. The
adapter validates the requested operation, calls `spec::pipeline::execute`, and
returns S05's `VerificationOutput`. It introduces no duplicate processing schema.

| Registered subject | Concrete request | Output |
|---|---|---|
| `spec:resize:request:v1` | `Request::Resize` | RGBA8 |
| `spec:perturb:request:v1` | `Request::Perturb` | RGBA8 |
| `spec:quantize:request:v1` | `Request::Quantize` | Indexed8 |
| `spec:dither-and-quantize:request:v1` | `Request::DitherAndQuantize` | Indexed8 |
| `spec:process:request:v1` | `Request::Process` | Indexed8 |

Every supported mode remains a typed request parameter. This includes resize
anchors/support, matching metrics, alpha policies, fields/seeds, working spaces,
placement, diffusion feedback/scan direction, and Yliluoma size. The registry
does not replace those choices with a limited list of hard-coded fixtures.

Indexed adapters retain logical indices, palette order, transparency, warning
codes, warning text, and warning order. RGBA adapters retain logical bytes without
row padding. Separable compositions use the actual rounded/clipped RGBA8 stage;
the adapter does not reconstruct that stage from unrounded coordinates.

## Seven f32 color pairs

The registry also includes `spec:color:<space>:f32-roundtrip-v1` for `srgb`,
`linear-rgb`, `oklab`, `oklch`, `cielab`, `cielch`, and `ycbcr`.
These accept `ReferenceRequest::Color { source, space }`. A subject rejects a
request carrying another space.

Each pair calls `spec::color::rgb8_to_coordinates` and its matching
`spec::color::coordinates_to_rgb8` inverse. Results contain packed f32 triplets,
separate byte alpha, the explicit space, and actual inverse-rendered RGBA8.
These are the named f32 conversions. Perturbation's wide f64 reconstruction is
different arithmetic and is not substituted into color review images.

## Case identity and conformance

Build cases through the existing S05 helpers:

1. Validate `ReferenceRequest::dimensions()` before constructing the case.
2. Hash `ReferenceRequest::source()` dimensions and complete bytes with `input_digest`.
3. Hash the typed `ReferenceRequest` with `settings_digest`. Its `Serialize`
   implementation serializes settings only, not a complete request transport.
4. Use `ReferenceRequest::semantics()` and the registered function in
   `VerificationSubject<ReferenceRequest>`. Supply the actual tested artifact's
   full revision and content digest, then call `evaluate`.

Settings serialization retains the caller palette order and every recipe field.
`SemanticIdentity.space` means the primary coordinate space for color/perturb,
or the matching space for indexed operations. It is not the entire stage recipe.
A process may perturb in Oklab and match in CIELAB. Both tags remain in its
settings digest, even though the primary tag is CIELAB. Resize has no primary tag.

S17 references remain `ReferenceState::PreFreeze`. Registration does not establish
the frozen checkpoint. S18 supplies that checkpoint and its validation.
Missing accepted/candidate adapters remain absent required roles. Reference code
must not fill those roles or stand in for unavailable production implementations.

The focused examples in `tests/reference_subjects.rs` exercise registry lookup,
S05 records, absent roles, all mode families, exact composition, and f32 inverses.
They collect no timings. Fresh performance orchestration remains a separate task.
