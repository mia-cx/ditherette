// Read retained paired results. Print compact audit data without modifying evidence.
// Usage: node .plans/83-scalar-report-audit.mjs <results-directory> [...]
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';

const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const hex = (bytes) => Buffer.from(bytes).toString('hex');
const median = (values) => {
  const sorted = values.toSorted((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
};
const countBy = (values) => values.reduce((counts, value) => {
  counts[value] = (counts[value] ?? 0) + 1;
  return counts;
}, {});

function audit(directory) {
  const read = (name) => readFileSync(resolve(directory, name));
  const reportBytes = read('report.json');
  const preparedBytes = read('prepared.json');
  const eventBytes = read('events.jsonl');
  const report = JSON.parse(reportBytes);
  const prepared = JSON.parse(preparedBytes);
  const events = eventBytes.toString().trim().split('\n').map((line) =>
    JSON.parse(line.replace(/"unix_ns":(\d+)/, '"unix_ns":"$1"')));
  const live = new Map();
  const reaped = new Map();
  let maximumLive = 0;
  let previous = 0n;
  for (const event of events) {
    assert(BigInt(event.unix_ns) >= previous, 'event timestamps must be ordered');
    previous = BigInt(event.unix_ns);
    if (event.state === 'started') {
      assert.equal(live.size, 0, 'workers must execute serially');
      assert(!reaped.has(event.trial), 'each trial starts once');
      live.set(event.trial, event.pid);
      maximumLive = Math.max(maximumLive, live.size);
    } else if (event.state === 'reaped') {
      assert.equal(live.get(event.trial), event.pid, 'reap must match its started pid');
      live.delete(event.trial);
      reaped.set(event.trial, event.pid);
    } else assert.equal(event.state, 'starting');
  }
  assert.equal(live.size, 0, 'all workers must be reaped');
  const expectedOrder = Array.from({ length: prepared.experiment.pairs }, (_, pair) =>
    prepared.experiment.cases.flatMap((_, index) => (pair % 2 ? ['candidate', 'accepted'] : ['accepted', 'candidate'])
      .map((role) => `pair-${String(pair).padStart(3, '0')}-case-${String(index).padStart(3, '0')}-${role}`))).flat();
  assert.deepEqual(events.filter((event) => event.state === 'started').map((event) => event.trial), expectedOrder);
  const byCase = new Map();
  const builds = new Map();
  const workerDigests = createHash('sha256');
  for (const name of readdirSync(directory).filter((name) => name.endsWith('.result.json')).sort()) {
    const bytes = read(name);
    const worker = JSON.parse(bytes);
    assert.equal(reaped.get(name.replace('.result.json', '')), worker.pid);
    assert.equal(worker.max_live_benchmark_processes, 1);
    assert.equal(worker.build.dirty, false);
    assert.equal(worker.iterations_per_sample, 1, 'this audit expects single-call observations');
    assert(worker.sample_ns.length >= 5 && worker.sample_ns.length <= worker.measurement.samples);
    assert(worker.sample_ns.every((value) => Number.isFinite(value) && value >= 0));
    const identity = prepared[worker.role].identity;
    assert.equal(worker.build.revision, identity.revision);
    assert.deepEqual(worker.output.implementation.artifact, identity);
    workerDigests.update(`${name}\0${digest(bytes)}\n`);
    builds.set(worker.build.revision, worker.build);
    const workers = byCase.get(worker.case_name) ?? [];
    workers.push(worker);
    byCase.set(worker.case_name, workers);
  }
  const cases = report.cases.map((entry) => {
    const recipe = prepared.experiment.cases.find((item) => item.name === entry.case_name);
    const workers = byCase.get(entry.case_name);
    assert.equal(workers.length, prepared.experiment.pairs * 2);
    const samples = Object.fromEntries(['accepted', 'candidate'].map((role) => {
      const selected = workers.filter((worker) => worker.role === role);
      assert.equal(new Set(selected.map((worker) => worker.pair)).size, prepared.experiment.pairs);
      const values = selected.flatMap((worker) => worker.sample_ns);
      assert.equal(median(values), entry[`${role}_median_ns`]);
      return [role, { count: values.length, per_worker: selected.map((worker) => worker.sample_ns.length) }];
    }));
    const actualPairRatios = Array.from({ length: prepared.experiment.pairs }, (_, pair) => {
      const roleMedian = (role) => median(workers.find((worker) => worker.pair === pair && worker.role === role).sample_ns);
      return roleMedian('candidate') / roleMedian('accepted');
    });
    actualPairRatios.forEach((ratio, i) => assert(Math.abs(ratio - entry.pair_ratios[i]) < 1e-12));
    assert.equal(entry.median_ratio, entry.candidate_median_ns / entry.accepted_median_ns);
    const mismatches = entry.verification.flatMap((proof) =>
      ['reference_accepted', 'reference_candidate', 'accepted_candidate']
        .filter((role) => !proof[role].exact)
        .map((role) => ({ role, ...proof[role] })));
    return { name: entry.case_name, gate: entry.gate, accepted_ns: entry.accepted_median_ns,
      candidate_ns: entry.candidate_median_ns, candidate_over_accepted: entry.median_ratio,
      pair_ratios: entry.pair_ratios, resolution_limited: entry.resolution_limited,
      samples, scope: recipe.measurement.scope, source: recipe.source, output: recipe.identity.output,
      subjects: [recipe.accepted_subject, recipe.candidate_subject],
      verification_statuses: countBy(entry.verification.map((proof) => proof.status)),
      mismatch_count: mismatches.length, mismatches: [...new Set(mismatches.map(JSON.stringify))].map(JSON.parse) };
  });
  const workers = [...byCase.values()].flat();
  assert.equal(workers.length, reaped.size);
  assert.equal(cases.length, byCase.size);
  return { directory: resolve(directory), gate: report.gate, gates: countBy(cases.map((entry) => entry.gate)),
    hashes: { report: digest(reportBytes), prepared: digest(preparedBytes), events: digest(eventBytes),
      worker_inventory: workerDigests.digest('hex') },
    artifacts: Object.fromEntries(['accepted', 'candidate'].map((role) => [role,
      { revision: prepared[role].identity.revision, sha256: hex(prepared[role].identity.content) }])),
    machine: prepared.machine, builds: [...builds.values()], host_load_notes: prepared.experiment.host_load_notes,
    declared_measurements: [...new Set(prepared.experiment.cases.map((item) => JSON.stringify(item.measurement)))].map(JSON.parse),
    workers: workers.length, samples: workers.reduce((total, worker) => total + worker.sample_ns.length, 0),
    events: countBy(events.map((event) => event.state)), maximum_live_workers: maximumLive,
    started_unix_ns: events[0].unix_ns, reaped_unix_ns: events.at(-1).unix_ns,
    first_start_utc: new Date(Number(BigInt(events[0].unix_ns) / 1000000n)).toISOString(),
    final_reap_utc: new Date(Number(BigInt(events.at(-1).unix_ns) / 1000000n)).toISOString(), cases };
}

assert(process.argv.length > 2, 'supply at least one result directory');
console.log(JSON.stringify(process.argv.slice(2).map(audit)));
