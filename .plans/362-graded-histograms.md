# #362 Add the graded pipeline stage and clipping indicators to histograms

## Summary
The Scopes window plots Source and Output. Add the full-colour image after the enabled effects,
before resize and palette matching, as a third stage. Any two stages overlay for comparison, and
the histogram reports pure-black and pure-white clipping for the stage drawn in colour.

## Acceptance criteria
- [x] Source, Effects and Output can each be inspected, alone or against one other stage.
- [x] The Effects stage is each source pixel's colour through the effects worker's current results,
      so masked, mixed and analysing chains match the output's input exactly.
- [x] Effects histograms update as effects change, in every model including RGB and YCbCr luma.
- [x] Clipping counts only pixels with all three channels at 0 or 255, skips fully transparent
      pixels, keeps partially transparent ones, and reads the stage drawn in colour.
- [x] Results for older settings or another source never show as current: the scope dims, reads
      Updating… and hides clipping values until the current results arrive.
- [x] Sampling stays bounded at 262,144 pixels, reuses the source's sample positions, and shows
      shares rather than counts. No canvas readback and no new image cache.
- [x] Old saved Source/Output scope settings load unchanged.

## TODOs
- [x] Stage model: `src/lib/scopes/stages.ts` (Effects stage store, effected sampling, stage
      picking) and `clipping`/share formatting in `scopes.ts`, with node specs.
- [x] Panel: three stage toggles (at most two shown), clipping footer, pending state; rename the
      scope components' `source`/`output` props to `under`/`over`; optional `effects` setting.
- [x] Component spec for saved settings, stage switching, clipping and pending.
- [x] DESIGN.md Scopes section and a website changeset.
- [x] Validation: focused unit/browser specs, existing scope specs, check, lint, build.
- [x] Before/after desktop and phone media on base and candidate.

## Notes
- Base: exact main `fb3251a4ea0923429869d45555279aa185cd7a99`, then empty-rebased to `621b439dc335446fd521da2562bcc290cbaceb6b`, branch `t3/graded-histogram-clipping`.
- No GitHub Project is attached to the issue.
- Effects are pointwise per colour (`compileEffects`): the worker's `results[index & 0xffffff]` with
  the pixel's own alpha (`index >>> 24`) is exactly what processing maps the source through.
- `compiledEffects` can hold an older key as a stand-in; currentness is `key === currentEffectsKey()`
  and `source === sourceImageData`. Output currentness is `exportable`.
- Parent owns DIRECTION.md, merge and deployment.
- Evidence directory: `/home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/graded-histograms-362/`.
- Panel samples per stage behind boolean gates, so scope/model switches, currentness flips and
  processing start/finish never resample (instrumented live: zero resamples; one compile result
  is one `sampleEffected`). `effectsStage` drops its retained reference when nobody listens.
- Validation: node scope specs 35/35, ScopesPanel browser spec 4/4, ExportStrip 1/1,
  live-effects 28/28, `pnpm check` 0 errors, `pnpm lint`, `pnpm build:web`, `git diff --check`.
- A key-blind `current` mutation fails 2 stage specs and 1 panel spec.
- An effects compile failure leaves the stage reading Updating…, like the Source preview keeps
  its last result.
