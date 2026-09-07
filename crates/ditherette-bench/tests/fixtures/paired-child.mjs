#!/usr/bin/env node
// Fixed responses exercise control flow only. This fixture takes no timings.
import assert from 'node:assert/strict';
import { closeSync, openSync, readFileSync, unlinkSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const request = JSON.parse(readFileSync(process.argv[3], 'utf8'));
assert.equal(process.argv[2], 'paired-trial');
assert.ok(Number(process.env.DITHERETTE_BENCH_LEASE_FD) >= 3);
assert.equal(spawnSync('flock', ['-n', '/tmp/ditherette-bench.lock', 'true']).status, 1);
const overlap = `${process.env.DITHERETTE_PAIR_FIXTURE_DIRECTORY}/active`;
const descriptor = openSync(overlap, 'wx');
process.on('exit', () => { closeSync(descriptor); unlinkSync(overlap); });
if (process.env.DITHERETTE_PAIR_FIXTURE_FAILURE === 'exit') process.exit(7);
if (process.env.DITHERETTE_PAIR_FIXTURE_FAILURE === 'json') {
  process.stdout.write('broken json');
  process.exit(0);
}
const subject = request.role === 'accepted' ? request.case.accepted_subject : request.case.candidate_subject;
const output = { case: request.case.identity,
  implementation: { subject, artifact: request.executable },
  output: { dimensions: request.case.identity.output, pixels: { format: 'rgba8', data: request.case.rgba }, warnings: [] } };
const reference = structuredClone(output);
reference.implementation.subject = request.case.reference_subject;
process.stdout.write(JSON.stringify({ role: request.role, pair: request.pair, case_name: request.case.name,
  build: { revision: request.executable.revision, dirty: false, rustc: 'fake compiler', tool_version: 'fake' },
  measurement: request.case.measurement, warmup_iterations: 1, warmup_elapsed_ns: 1,
  sample_ns: [100, 100, 100, 100, 100], iterations_per_sample: 1,
  reference, output, pid: process.pid, max_live_benchmark_processes: 1 }));
