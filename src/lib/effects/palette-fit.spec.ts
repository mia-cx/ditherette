import { describe, expect, it } from 'vitest';
import { ADDABLE_KINDS, EFFECTS } from './catalog';
import { NUMBER_FIELDS } from '../../routes/components/effects/fields';
import { decideLockedFit, fitFingerprint, type FitInputs, type FitState } from './palette-fit';

const exposure = EFFECTS.exposure.create();

/** A layer needs only its step here; tests add identity fields the fingerprint must ignore. */
interface Holder {
	readonly id?: string;
	readonly name?: string;
	readonly step: import('ditherette').Effect;
}

const rgb = (n: number) => ({ r: n, g: n, b: n });
const values = { '#000000': rgb(0), '#FFFFFF': rgb(255), transparent: null };

const inputs = (overrides: Partial<FitInputs<Holder>> = {}): FitInputs<Holder> => ({
	paletteName: 'Test palette',
	enabled: { '#000000': true, '#FFFFFF': true },
	values,
	crop: undefined,
	before: [{ step: exposure }],
	...overrides
});

describe('catalog', () => {
	it('adds a neutral fitted palette fit and keeps recolour loadable but unlisted', () => {
		expect(EFFECTS['palette-fit'].create()).toEqual({
			effect: 'palette-fit',
			enabled: true,
			look: 'fitted',
			space: 'oklab',
			strength: 1,
			curves: null
		});
		expect(EFFECTS.recolour.label).toBe('Palette fit (legacy)');
		expect(ADDABLE_KINDS).toContain('palette-fit');
		expect(ADDABLE_KINDS).not.toContain('recolour');
	});

	it('lets palette fit strength reach 300% while recolour stays at 100%', () => {
		expect(NUMBER_FIELDS['palette-fit'][0]).toMatchObject({ key: 'strength', max: 300 });
		expect(NUMBER_FIELDS.recolour[0]).toMatchObject({ key: 'strength', max: 100 });
	});
});

describe('fitFingerprint', () => {
	it('reads the palette selection, crop, and enabled steps before the fit', () => {
		const base = inputs();
		expect(fitFingerprint({ ...base, paletteName: 'Other' })).not.toBe(fitFingerprint(base));
		expect(fitFingerprint({ ...base, enabled: { ...base.enabled, '#FFFFFF': false } })).not.toBe(
			fitFingerprint(base)
		);
		expect(fitFingerprint({ ...base, values: { ...values, '#000000': rgb(64) } })).not.toBe(
			fitFingerprint(base)
		);
		expect(fitFingerprint({ ...base, crop: { x: 1, y: 2, width: 3, height: 4 } })).not.toBe(
			fitFingerprint(base)
		);
		expect(fitFingerprint({ ...base, before: [{ step: { ...exposure, stops: 1 } }] })).not.toBe(
			fitFingerprint(base)
		);
	});

	it('ignores layer identity and names, and skips disabled steps before the fit', () => {
		const base = inputs();
		const renamed = inputs({
			before: [{ id: 'x', name: 'A name', step: exposure }],
			enabled: base.enabled
		});
		expect(fitFingerprint(renamed)).toBe(fitFingerprint(base));
		const disabled = inputs({
			before: [{ step: { ...exposure, enabled: false } }]
		});
		expect(fitFingerprint(disabled)).toBe(fitFingerprint({ ...base, before: [] }));
	});
});

describe('decideLockedFit', () => {
	const edited = [
		{
			kind: 'remap' as const,
			x: { model: 'oklch' as const, channel: 'lightness' as const },
			y: { model: 'oklch' as const, channel: 'lightness' as const },
			points: [
				[0, 0],
				[1, 0.5]
			] as [number, number][]
		}
	];

	it('keeps the fit locked while its inputs hold', () => {
		const state: FitState<Holder> = { inputs: inputs() };
		expect(decideLockedFit(edited, state, inputs())).toEqual({
			action: 'unchanged',
			state
		});
	});

	it('re-analyses on changed inputs and keeps one revert snapshot', () => {
		const locked = inputs();
		const next = inputs({ paletteName: 'Other' });
		const decision = decideLockedFit(edited, { inputs: locked }, next);
		expect(decision.action).toBe('reanalyse');
		expect(decision.state.inputs).toBe(next);
		expect(decision.state.revert).toEqual({ inputs: locked, curves: edited });
	});

	it('keeps Revert when a different palette is selected, drops it for a colour edit', () => {
		const locked = inputs();
		const switched = decideLockedFit(edited, { inputs: locked }, inputs({ paletteName: 'Other' }));
		expect(switched.state.revert).toBeDefined();

		// Editing a colour inside the same palette re-analyses without a snapshot: Revert could
		// not restore the RGB values anyway.
		const recoloured = decideLockedFit(
			edited,
			{ inputs: locked },
			inputs({ values: { ...values, '#000000': rgb(64) } })
		);
		expect(recoloured.action).toBe('reanalyse');
		expect(recoloured.state.revert).toBeUndefined();
	});
});
