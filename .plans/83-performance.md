# #83 Complete-call release evidence

## Scope

Base is S40 PR129 `96c281180529c3849823736581daebdcdbd8493e`.
Branch is `impl/v1-s41-performance`. This worktree owns benchmark adapters,
subjects, matrix generation, preparation instructions, and this plan.
The coordinator owns artifact joins, exclusive timing, reports, and PR filing.
The cache-candidate agent owns production changes. Frozen spec/image stay unchanged.

## Acceptance

- [ ] Fixed preview/common/large/capped coverage includes direct, fields, diffusion, perceptual modes, and extras.
- [ ] Fresh comparisons preserve pre-S32 cold regression anchors and include faithful website TypeScript operations.
- [ ] Cold/warm complete calls retain copies, hashing, preparation, exact output verification, and achieved sample counts.
- [ ] Confirmed >10% regressions block release; unavailable or noisy evidence stays incomplete.
- [ ] The final report binds source, immutable artifacts, versions, wall time, and all remaining gates.

## Implementation TODOs

- [x] Add guarded actual-website direct/Process adapters and focused non-timing tests.
- [x] Generate bounded release plans using existing subjects and unchanged S32 anchors.
- [x] Validate matrix counts, provenance requirements, source identities, and wall-time estimates.
- [x] Hand off clean committed infrastructure with owned jobs drained.

## Measurement hold

No measurements until the coordinator explicitly grants the exclusive window.
Use two alternating pairs, at most 20 samples, at least five samples, and the
existing 10-second accumulated measured-time cap. Priming, verification, setup,
and capped transport also consume wall time. A slow call can exceed the cap.
Wait for process exit. Do not poll progress or retry outside the declared budget.
Threaded WebKit stays excluded because its atomic-wait cleanup gate remains open.

S40 ordinary package SHA-256 is
`1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
Its tested automatic-runtime source is `dc81818a`; these are conformance records,
not fresh S41 measurements. The accepted cold-anchor source remains pre-S32
`d51a70a2daf054357d16ea66b235da3733c02888`.

## TypeScript limits

S39 proves a conservative nearest/no-dither/sRGB/preserve-alpha subset.
New benchmark adapters reuse actual website algorithms and that admission rule.
General perceptual, field, diffusion, and mixing parity remains unproved.
Existing area/bilinear/Lanczos drift remains diagnostic evidence, not equivalence.
Bicubic, trilinear, and other genuinely absent website modes establish package baselines.

Adapter validation passes the offline closure/output/refusal test and 29 focused
native protocol tests. The output test covers duplicate palette order, explicit
transparency, durable results, nearest composition, and the real 2→49 mismatch.
No browser or processing timing calls run. TypeScript checking passes against the
retained S40 public declarations. An initial check without those declarations
fails on missing generated Wasm modules, not an adapter error.

## Fixed matrix and proposed wall-time budget

`release_integration_plan` generates one case per experiment file. First generate
its compact inventory; use that inventory's case indices. Each destination must
be new. Source arrays never appear in the inventory. Do not parse complete
prepared snapshots into Node merely to inspect counts or provenance.

```text
release_integration_plan LANE inventory NEW_INVENTORY_JSON HOST_LOAD_NOTES
release_integration_plan LANE CASE_INDEX NEW_CASE_JSON HOST_LOAD_NOTES
```

Every lane uses two alternating pairs and the existing sample protocol above.
The counts below include both roles and both pairs. Each measured worker exits
before the next starts. Parenthesized budgets include setup, frozen verification,
output observation, disposal, and untimed priming, not just method timers.

| Lane | Cases | Engines | Workers | Proposed wall budget |
| --- | ---: | --- | ---: | ---: |
| candidate / candidate-native | 2 each | C/F/W scalar plus native | 32 | 10 min |
| anchors / anchors-native | 8 each | C/F/W scalar plus native | 128 | 20 min |
| release | 15 | C/F/W scalar | 180 | 60 min |
| automatic | 10 | C/F host-worker | 80 | 40 min |
| typescript | 3 | C/F/W page, both roles | 36 | 10 min |
| initialization / initialization-threads | 2 each | C/F/W scalar; C/F threads | 40 | 15 min |
| capped | 1 | C/F/W scalar, feasibility-held | 12 | 60 min provisional |
| Total | | | 508 | 215 min provisional |

Reserve another 30–60 minutes for fresh immutable build preparation and untimed
resource checks. These are planning estimates, not measured upper bounds. The
coordinator approves lanes before running them, stops launching new work at the
agreed wall deadline, and lets owned work exit. Unrun cases remain incomplete.
Do not launch the full matrix blindly or repeat a noisy lane without a bounded
experiment decision. Start with the two cold candidate anchors.

The retained S35–S37 400-worker trial took 35 minutes in Chromium and 141 minutes
in Firefox. Firefox's scalar 769×513 random field takes about 24.5 seconds per
call. Its cold worker therefore exceeds the nominal 10-second measured cap to
achieve five samples. Twenty untimed same-call primes would cost about eight
minutes per worker, so this matrix does not repeat that expensive final-hit case.
The medium Yliluoma final-hit control retains priming and remains separately visible.

### Coverage

Release recipes include preview 512×384 nearest and Sierra/Oklab Process,
3.15 MP common direct and Floyd–Steinberg Process, and 8.39 MP large nearest.
The selected area/bilinear/Lanczos3 classes reuse measured shapes without a new
threshold sweep. Random perturb stays 769×513; blue/adaptive Oklab64 stays 65×49.
Medium palette8 Yliluoma and nearest+OKLCH Yliluoma Process reuse S37 recipes.
Trilinear establishes an extra-mode baseline. Preview nearest and medium mixing
have final-hit controls. Historical anchors retain four cold and four warm
recipes, including partial perturb, resize, and no-dither primes.

The automatic lane compares ordinary scalar versus ordinary required-threaded
packages, with no developer row policy. Its eight cold cases cover the three selected
resize classes, direct matching, random perturb, blue/adaptive fused processing,
mixing, and mixing Process. Two warm controls cover cheap scalar fallback and
medium mixing. These timings do not select new size thresholds.

The TypeScript lane uses actual website nearest, direct quantize, and no-dither
Process. Indexed fixtures contain exact palette colors and explicit transparency.
They establish only that admitted workload class. Both roles run on the page;
TypeScript remains stateless and makes no application-cache claim. General
website field/diffusion/perceptual comparisons remain unavailable, not spec-only.

### Capped feasibility hold

Required capped input is 1×1, producing 8192×8192 indexed pixels through nearest
Process. It exercises maximum output area without embedding a maximum-area RGBA
source in every JSON request. It is not a maximum-source-image benchmark.

The current collector's 64 MiB stability budget rejects this 64 MiB result before
timing. Maximum-area RGBA is also impossible in the existing JSON transport: even
single-digit byte entries need 536,870,913 characters, exceeding this Node's
536,870,888-character string limit. S35 owns the separate bounded feasibility fix.
The maximum-area case remains required and held until its untimed resource check
passes. A 12 MiB substitute does not satisfy that gate. No global limit increase
or production change is part of this matrix.

## Artifact bindings and execution order

1. Root joins this infrastructure into clean accepted/current and candidate
   snapshots. Current production is S40 `96c28118`; its later CI-only commits do
   not change runtime inputs. Candidate adds only `6e48b7dd471edec562a9b19aa53c3688d250d423`.
   Audit the production diff before building. Keep that candidate unselected.
2. Root prepares the historical `d51a70a2` snapshot with shared protocol-only
   updates where required. Its production source remains pre-S32. Never replace
   this acceptance anchor with a later regressed implementation.
3. Run the existing fresh native/public preparation scripts in each snapshot.
   Public preparation omits `--bench-subjects`: these are ordinary packages.
   Bind native binaries, package assets, TS closure, frozen oracle, source hashes,
   tool/browser versions, and per-case input/settings identities. Build before
   quiet clearance. Follow `crates/ditherette-bench/PAIRED.md` for commands.
4. Candidate/release lanes compare current S40 against the one exact candidate.
   Anchors compare pre-S32 against the final selected runtime. Automatic compares
   scalar/threads of that selected ordinary artifact. TS compares the actual
   website closure against that artifact. Initialization compares current/final
   with compilation scopes separate. Capped compares current/final only after
   its resource gate. If the candidate loses, keep the accepted runtime and
   report that decision; do not relabel candidate output as the baseline.
5. Drain agents/builds/tests. Root owns the shared lease, serial execution,
   achieved sample counts, exact result audit, and final report. No threaded
   WebKit lane runs. Existing frozen resize drift remains diagnostic and incorrect;
   missing/noisy required comparisons and inherited cold regressions stay open.

## Infrastructure checkpoint validation

- 13 release-generator tests pass, including reused frozen composition tests,
  fixed lane counts, unchanged cold-anchor identities, and capped dimensions.
- 29 focused native browser/quantize protocol tests pass, plus the library check.
- 20 JavaScript closure, fake-clock collector, and stage-prime tests pass.
- TypeScript adapter checking passes using retained S40 public declarations.
- The generator emits a compact candidate inventory and one candidate case under
  this worktree's ignored `target/`. It executes no processing call or benchmark.
- No production, frozen spec/image, or frozen-guard files change.

The only owned compiler output is this worktree's `target/compiler`. Root owns
cleanup after handoff. Preserve generated plans and any future retained evidence.
No Wasm build, browser launch, real measurement, push, or PR runs here.

CI-only S40 head `95706738e4f3179824a66c80bb5728ce97a34b4b` joins through
`4e0018290311fabe1c6e1dd47d39ee579adfad68`. Runtime inputs remain unchanged.
Adapter checkpoint is `9b06037d`; matrix checkpoint is `37e8cc8b`.
All owned native/JavaScript jobs exit before this handoff. The compiler directory
uses approximately 1.7 GiB and returns to root ownership without deletion.
Generated inventory SHA-256 is
`4d507afe536a8db31c2d150156c24184ed898d46c46faa1aa421c4dacdb1e140`;
the first candidate case is
`d88c14b90a8e6f70637f8409e1205a57389b43e55d4972eeeae14e22d68ec996`.
These are untimed generator outputs, not prepared or measured artifact identities.

## Capped transport integration

Root reassigns this worktree's compiler cache for the bounded protocol integration.
S41 owns the optional Rust case field, constructor defaults, release matrix, and
page forwarding/typed comparisons. S35 owns collector limits, compact indexed
wire helpers/decoders, oracle wrappers, and asset registration.

- [x] Add `retained_output_limit_bytes`, omitted rather than null when unused.
- [x] Restrict overrides to >64–384 MiB for cold, fresh, single indexed package complete calls without priming.
- [x] Set only the required capped case to 384 MiB; old anchor metadata stays absent.
- [x] Forward the explicit bound and compact indexed evidence through the page.
- [x] Join S35's helper checkpoint and check the small exact page protocol end to end.

Thirty focused native protocol tests pass. All example constructors compile.
The inherited unused `self` import warning in stage/progress examples remains.
This confirms protocol structure, not a successful maximum-area allocation or measurement.

S35's collector/helper commits `337c09bb` and `7d14aa48` join as `31c94927` and
`f43fae28`. The first cherry-pick encounters a modify/delete conflict because this
branch lacks S35's earlier plan; retaining `.plans/83-capped-transport.md` resolves
it without changing implementation. Metadata checkpoint is `81677d3b`.

The compact page path validates its explicit limit before producing results,
decodes frozen indices once, compares typed preflight bytes, and emits compact
actual/mismatch/instability evidence. Composition errors stay bounded on this
path. Normal requests keep the old array transport and omit the new field.
Warm or primed overrides fail because a fourth maximum-area envelope exceeds
the retained three-output transport estimate.

Forty-six focused JavaScript tests pass, including tiny exact Process, staged
composition, frozen mismatch with no samples, A/B/A evidence, rejected limits,
and rejected prime evidence. Thirty native protocol tests and the allocation-free
capped-generator test pass. The page uses fake package methods and a fake clock;
these are not performance samples. S35's decoder/oracle/asset-registration join
and root's quiet-window maximum-area resource check remain separate requirements.
Regenerating the first cold candidate request after adding metadata produces
byte-identical JSON to `target/s41-candidate-0.json` (`cmp` passes).
