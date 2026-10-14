import { createRequire } from 'node:module';
import { compile } from 'tailwindcss';

const require = createRequire(import.meta.url);

// Include Typography's default selectors and cases its pseudo-element lifting must preserve.
const customSelectors = [
	'ol li::before, ul li::before',
	'ol li::marker::before, ul li::marker::before',
	'ol li::before, ul li::after',
	'ol li::scroll-thumb::before, ul li::scroll-track::before',
	'ol li::before::scroll-thumb, ul li::before::scroll-track',
	'> :is(ol, ul) > li:nth-child(2n + 1)::marker',
	':is(a:has(> img:not([hidden])), blockquote:not(.quiet))::before',
	String.raw`.hover\:underline:hover::before, .escaped\,name::before`,
	String.raw`.\31 st::after, .caf\e9::after`,
	'[data-note="a,b"]::before, [data-note="c:d"]::before',
	'svg|a[href^="https://"]:not([rel~="external"])::after',
	'li/* annotation */::marker',
	`${':not('.repeat(32)}a${')'.repeat(32)}::before`
];

function selectorsFrom(styles) {
	return Object.entries(styles).flatMap(([selector, value]) =>
		value && typeof value === 'object' && !Array.isArray(value)
			? [selector, ...selectorsFrom(value)]
			: []
	);
}

// Source positions change across parser versions; compare selector meaning, escaping and spacing.
function selectorAst(node) {
	const fields = [
		'type',
		'value',
		'attribute',
		'operator',
		'namespace',
		'insensitive',
		'quoteMark',
		'raws',
		'spaces'
	];
	return {
		...Object.fromEntries(
			fields.filter((key) => node[key] !== undefined).map((key) => [key, node[key]])
		),
		...(node.nodes ? { nodes: node.nodes.map(selectorAst) } : {})
	};
}

/** Capture the real Typography parser calls and Tailwind 4 prose output for a dependency comparison. */
export async function captureTypography(entrypoint = require.resolve('@tailwindcss/typography')) {
	const typographyRequire = createRequire(entrypoint);
	const typography = typographyRequire(entrypoint);
	const { commonTrailingPseudos } = typographyRequire('./utils');
	const parser = typographyRequire('postcss-selector-parser');
	const styles = typographyRequire('./styles');
	const selectors = [
		...new Set([
			...Object.values(styles).flatMap(({ css }) => [css].flat().flatMap(selectorsFrom)),
			...customSelectors,
			'&:hover::before'
		])
	];
	const parsed = selectors.map((selector) => ({
		selector,
		trailingPseudos: commonTrailingPseudos(selector),
		ast: selectorAst(parser().astSync(selector)),
		roundTrip: parser().processSync(selector)
	}));
	const cases = [];
	for (const className of ['prose', 'markdown']) {
		const custom = Object.fromEntries(
			customSelectors.map((selector) => [selector, { color: 'red' }])
		);
		const config = {
			theme: {
				extend: {
					typography: {
						DEFAULT: {
							css: {
								...custom,
								a: { '&:hover': { color: 'blue' } }
							}
						}
					}
				}
			}
		};
		const compiled = await compile(
			`
			@theme { --color-blue-500: blue; --breakpoint-md: 48rem; }
			@custom-variant dark (&:is(.dark *));
			@config "typography-fixture";
			@plugin "@tailwindcss/typography";
			@tailwind utilities;
		`,
			{
				loadModule: async (id, base, kind) => ({
					path: entrypoint,
					base,
					module: kind === 'plugin' ? typography({ className }) : config
				})
			}
		);
		cases.push({
			className,
			css: compiled.build([
				className,
				...['sm', 'lg', 'xl', '2xl', 'slate', 'invert'].map(
					(modifier) => `${className}-${modifier}`
				),
				`dark:${className}-invert`,
				`md:${className}-lg`,
				`${className}-headings:underline`,
				`${className}-a:text-blue-500`,
				`hover:${className}-a:underline`,
				`${className}-code:before:underline`
			])
		});
	}
	return {
		versions: {
			typography: typographyRequire('../package.json').version,
			parser: typographyRequire('postcss-selector-parser/package.json').version,
			tailwind: require('tailwindcss/package.json').version
		},
		parsed,
		cases
	};
}
