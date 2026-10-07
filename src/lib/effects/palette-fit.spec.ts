import { describe, expect, it } from 'vitest';
import { ADDABLE_KINDS, EFFECTS } from './catalog';
import { decideLockedFit, fitFingerprint, type FitInputs, type FitState } from './palette-fit';

const exposure = EFFECTS.exposure.create();

/** A layer needs only its step here; tests add identity fields the fingerprint must ignore. */
interface Holder {
	readonly id?: string;
	readonly name?: string;
	readonly step: import('ditherette').Effect;
}

const inputs = (overrides: Partial<FitInputs<Holder>> = {}): FitInputs<Holder> => ({
	paletteName: 'Test palette',
	enabled: { '#000000': true, '#FFFFFF': true },
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
});

describe('fitFingerprint', () => {
	it('reads the palette selection, crop, and enabled steps before the fit', () => {
		const base = inputs();
		expect(fitFingerprint({ ...base, paletteName: 'Other' })).not.toBe(fitFingerprint(base));
		expect(fitFingerprint({ ...base, enabled: { ...base.enabled, '#FFFFFF': false } })).not.toBe(
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

		// A further change replaces the snapshot rather than stacking.
		const third = inputs({ paletteName: 'Third' });
		const again = decideLockedFit(edited, decision.state, third);
		expect(again.action).toBe('reanalyse');
		expect(again.state.revert).toEqual({ inputs: next, curves: edited });
	});
});
