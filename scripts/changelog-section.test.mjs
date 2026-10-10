import assert from 'node:assert/strict';
import { test } from 'node:test';
import { changelogSection } from './changelog-section.mjs';

const changelog = `# ditherette

## 1.2.3

### Patch Changes

- Fixes the thing.

## 1.2.2

- Older.

## 1.0.0-rc.1

- Prerelease.
`;

test('changelogSection returns the requested version body', () => {
	assert.equal(changelogSection(changelog, '1.2.3'), '### Patch Changes\n\n- Fixes the thing.');
	assert.equal(changelogSection(changelog, '1.2.2'), '- Older.');
});

test('changelogSection rejects missing and empty sections', () => {
	assert.throws(() => changelogSection(changelog, '9.9.9'), /no section for 9\.9\.9/);
	assert.throws(() => changelogSection('## 1.0.0\n', '1.0.0'), /empty/);
});
