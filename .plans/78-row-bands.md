# S36 quantize and separable-field row bands

Issue #78. Branch `impl/v1-s36-fields` starts at validated S34 delivery
`3863cadce82b4d272a1730f3ebad828b18ac4434` in `.worktrees/v1-s36-fields`.
Read the current coordinator execution contract, S36 slice, reuse inventory
`78-79-reuse-inventory.md`, and decisions 24/37/40 before implementation.

## Reuse and ownership

Keep `PreparedQuantizer`, its ordered palette/matcher, and packed converter.
Its allocation-free pixel loop already implements alpha and stable tie rules.
Add a disjoint-output row adapter by extracting that existing row loop once.
The source remains a full immutable view; metadata stays owned by preparation.

Keep the landed perturb arithmetic, field generators, adaptive placement, and
RGBA8 reconstruction. Its existing row method addresses a full output image.
Add a band-local output method with explicit absolute rows. Both methods call
the same pixel loop. Adaptive neighbors still read the original full source;
field identities remain `y * full_width + x`, even for suppressed pixels.
Keep the RGBA8 boundary before separable quantization. Diffusion remains scalar.

This worktree owns `prod/quantize/prepared.rs`, quantize row adapters,
`prod/dither/perturb.rs`, field row adapters, focused tests, an S36 benchmark
subject fragment, and this plan. Root reconciles shared pipeline/registry files.
S35 owns resize/color policy and shared scheduling work. Do not duplicate it.
No public API, execution selector, cache identity, or callback contract changes.

## Steps

1. [ ] Add safe band-local kernel adapters and prove exact bytes across disjoint
   output bands, worker counts, strides, alpha/ties, field identities, and adaptive seams.
2. [ ] Define bounded execution capacity using existing worker-budget geometry.
   Count each concurrently live perturb converter and other temporary records;
   share immutable quantizer preparation. Prove exact budget and one-under behavior.
3. [ ] Extend existing benchmark subjects before selecting a parallel candidate.
   Reuse complete-call adapters, including copies/preparation, and coordinate
   shared registration with root. Keep thresholds unselected pending fresh evidence.
4. [ ] Join the shared pooled executor and integrate candidate row scheduling
   through private fragments. Report progress only after joined work on the caller.
5. [ ] After root quiet clearance, compare fresh complete-call host-worker artifacts.
   Select only measured wins, record scalar winners, and hand off a reviewable PR.

## Validation and budget

The first checkpoint is native exactness and budget evidence, not a speed claim.
Use only fresh local `target/compiler`. No Wasm builds or timing are authorized yet.
Frozen spec and shared image infrastructure remain unchanged. Compare against
frozen output plus the landed scalar path, including metadata and warnings.

Native pool fixtures vary one, two, and four workers and non-dividing band heights.
Exercise all matching tags, alpha modes, Bayer sizes/random/blue-noise fields,
working spaces, adaptive radius larger than a band, and strided source guards.
Keep test images small. A field callback witness proves one global draw per pixel.
Prepared quantization must allocate no per-worker palette or full-image color plane.

Measured crossover policy is deferred. Root predeclares experiment targets and
budget before exclusive runs; at most two candidate revisions follow the contract.
Main-JS preferred remains scalar, and required remains a capability error there.
Real threaded public calls run in a blocking-capable processing host worker.
