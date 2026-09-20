import { chromium, firefox, webkit } from 'playwright';

const engines = { chromium, firefox, webkit };

/** Select explicit CI engines without allowing a misspelled filter to pass an empty suite. */
export function browserEngines(environment = process.env) {
	const names =
		environment.DITHERETTE_TEST_ENGINES === undefined
			? Object.keys(engines)
			: environment.DITHERETTE_TEST_ENGINES.split(',');
	if (
		!names.length ||
		names.some((name) => !Object.hasOwn(engines, name)) ||
		new Set(names).size !== names.length
	)
		throw new Error('DITHERETTE_TEST_ENGINES must list unique chromium, firefox, or webkit names.');
	return names.map((name) => [name, engines[name]]);
}

/** A test-owned runtime override leaves the shared Playwright installation unchanged. */
export function browserLaunchOptions(name, environment = process.env) {
	return { executablePath: environment[`DITHERETTE_TEST_${name.toUpperCase()}_EXECUTABLE`] };
}
