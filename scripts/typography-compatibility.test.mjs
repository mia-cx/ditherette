import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import test from 'node:test';
import { captureTypography } from './typography-compatibility.mjs';

// Captured from parser 6 before the scoped upgrade. Do not refresh to hide a CSS or selector migration.
const baseline = JSON.parse(
	readFileSync(new URL('./fixtures/typography-compatibility.json', import.meta.url), 'utf8')
);
const report = await captureTypography();
const digest = (value) => createHash('sha256').update(value).digest('hex');

test('Typography resolves the patched parser through its own dependency path', () => {
	assert.equal(report.versions.parser, '7.1.6');
	assert.equal(report.versions.typography, baseline.baseline.typography);
	assert.equal(report.versions.tailwind, baseline.baseline.tailwind);
});

test('all default and extended selectors retain their pseudo lifting and round-trip output', () => {
	assert.equal(report.parsed.length, baseline.selectors);
	assert.equal(
		digest(
			JSON.stringify(
				report.parsed.map(({ selector, trailingPseudos, roundTrip }) => ({
					selector,
					trailingPseudos,
					roundTrip
				}))
			)
		),
		baseline.api
	);
});

test('selector AST values, nesting, escaping and spacing match parser 6', () => {
	assert.equal(
		digest(JSON.stringify(report.parsed.map(({ selector, ast }) => ({ selector, ast })))),
		baseline.ast
	);
});

test('Tailwind 4 prose and custom-class CSS remain byte-identical to parser 6', () => {
	assert.deepEqual(
		report.cases.map(({ className, css }) => ({
			className,
			bytes: Buffer.byteLength(css),
			sha256: digest(css)
		})),
		baseline.css
	);
});

test('Typography rejects excessive selector nesting through the patched parser guard', () => {
	const require = createRequire(import.meta.url);
	const { commonTrailingPseudos } = require('@tailwindcss/typography/src/utils.js');
	assert.throws(
		() => commonTrailingPseudos(`${':not('.repeat(300)}a${')'.repeat(300)}`),
		/nesting depth exceeds the maximum/
	);
});
