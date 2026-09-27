import { describe, expect, it } from 'vitest';
import { EFFECTS, EFFECT_KINDS, isEffectStep } from './catalog';

describe('isEffectStep', () => {
	it('accepts every neutral default', () => {
		for (const kind of EFFECT_KINDS) expect(isEffectStep(EFFECTS[kind].create())).toBe(true);
	});

	it('rejects saved steps editors could not render', () => {
		const curves = EFFECTS.curves.create();
		const levels = EFFECTS.levels.create();
		for (const step of [
			{ ...curves, points: undefined },
			{ ...curves, points: [[0, 0]] },
			{
				...curves,
				points: [
					[0.5, 0],
					[0.5, 1]
				]
			},
			{
				...curves,
				points: [
					[0, 0],
					[1, 1, 1]
				]
			},
			{ ...levels, gamma: '1' },
			{ ...levels, input: { black: 0 } },
			{ ...levels, extra: true },
			{ ...EFFECTS.exposure.create(), stops: Number.NaN },
			{ effect: 'blur', enabled: true }
		])
			expect(isEffectStep(step)).toBe(false);
	});
});
