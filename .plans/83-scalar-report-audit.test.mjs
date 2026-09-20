import assert from 'node:assert/strict';
import test from 'node:test';

import { auditLifecycle } from './83-scalar-report-audit.mjs';

const event = (state, trial, pid, unix_ns) => ({ state, trial, pid, unix_ns });

test('accepts the coordinator lifecycle for every expected trial', () => {
  assert.doesNotThrow(() => auditLifecycle([
    event('starting', 'trial-a', null, '1'),
    event('started', 'trial-a', 17, '2'),
    event('reaped', 'trial-a', 17, '3'),
    event('starting', 'trial-b', null, '4'),
    event('started', 'trial-b', 23, '5'),
    event('reaped', 'trial-b', 23, '6'),
  ], ['trial-a', 'trial-b']));
});

test('rejects a trailing starting event', () => {
  assert.throws(() => auditLifecycle([
    event('starting', 'trial-a', null, '1'),
    event('started', 'trial-a', 17, '2'),
    event('reaped', 'trial-a', 17, '3'),
    event('starting', 'trial-a', null, '4'),
  ], ['trial-a']), /complete lifecycle/);
});

test('rejects a starting event assigned to the wrong trial', () => {
  assert.throws(() => auditLifecycle([
    event('starting', 'trial-b', null, '1'),
    event('started', 'trial-a', 17, '2'),
    event('reaped', 'trial-a', 17, '3'),
  ], ['trial-a']), /wrong trial/);
});
