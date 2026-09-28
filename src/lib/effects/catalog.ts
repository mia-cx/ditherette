import type {
	ChannelCurveEffect,
	ColourChannel,
	CurvesEffect,
	Effect,
	ModelCurvesEffect,
	ModelCurvesModel
} from 'ditherette';

export type CurvePoints = CurvesEffect['points'];
/** A model whose three channels each get a curve: encoded RGB, or any `model-curves` model. */
export type CurveModel = 'srgb' | ModelCurvesModel;
export type ChannelName = ColourChannel['channel'];

/**
 * A curves layer: three curves in a model's channel order, or one arbitrary XY curve where an
 * input channel adjusts an output channel. Processing turns it into package steps.
 */
export type CurvesLayer =
	| {
			readonly effect: 'curves';
			readonly enabled: boolean;
			readonly model: CurveModel;
			readonly curves: readonly [CurvePoints, CurvePoints, CurvePoints];
	  }
	| {
			readonly effect: 'curves';
			readonly enabled: boolean;
			readonly model: 'xy';
			readonly x: ColourChannel;
			readonly y: ColourChannel;
			readonly points: CurvePoints;
	  };

/** What a layer stores: a package step, except that curves keep a curve per channel. */
export type LayerStep =
	| Exclude<Effect, CurvesEffect | ModelCurvesEffect | ChannelCurveEffect>
	| CurvesLayer;
export type EffectKind = LayerStep['effect'];
export type EffectOf<K extends EffectKind> = Extract<LayerStep, { effect: K }>;

type ChannelInfo = { readonly name: ChannelName; readonly label: string };
type ModelChannels = readonly [ChannelInfo, ChannelInfo, ChannelInfo];
const channel = (name: ChannelName, label: string): ChannelInfo => ({ name, label });
const RGB: ModelChannels = [
	channel('red', 'Red'),
	channel('green', 'Green'),
	channel('blue', 'Blue')
];
const LAB: ModelChannels = [
	channel('lightness', 'Lightness'),
	channel('a', 'a'),
	channel('b', 'b')
];
const LCH: ModelChannels = [
	channel('lightness', 'Lightness'),
	channel('chroma', 'Chroma'),
	channel('hue', 'Hue')
];

/** Every colour model a curves layer can use, with its channels in canonical order. */
export const CURVE_MODELS: readonly {
	readonly id: CurveModel;
	readonly label: string;
	readonly channels: ModelChannels;
}[] = [
	{ id: 'srgb', label: 'RGB', channels: RGB },
	{ id: 'linear-rgb', label: 'Linear RGB', channels: RGB },
	{
		id: 'hsl',
		label: 'HSL',
		channels: [
			channel('hue', 'Hue'),
			channel('saturation', 'Saturation'),
			channel('lightness', 'Lightness')
		]
	},
	{
		id: 'hsv',
		label: 'HSV',
		channels: [
			channel('hue', 'Hue'),
			channel('saturation', 'Saturation'),
			channel('value', 'Value')
		]
	},
	{ id: 'oklab', label: 'OKLab', channels: LAB },
	{ id: 'oklch', label: 'OKLCH', channels: LCH },
	{ id: 'cielab', label: 'CIELAB', channels: LAB },
	{ id: 'cielch', label: 'CIELCh', channels: LCH },
	{
		id: 'ycbcr',
		label: 'YCbCr',
		channels: [channel('luma', 'Luma'), channel('cb', 'Cb'), channel('cr', 'Cr')]
	}
];

/** Every channel an arbitrary XY curve can read or adjust, grouped by model. */
export const XY_CHANNELS: readonly { readonly label: string; readonly channel: ColourChannel }[] =
	CURVE_MODELS.flatMap((model) =>
		model.channels.map(({ name, label }) => ({
			label: `${model.label} · ${label}`,
			// Each model lists only its own channels, so every pair is a valid ColourChannel.
			channel: { model: model.id, channel: name } as ColourChannel
		}))
	);

export const STRAIGHT: CurvePoints = [
	[0, 0],
	[1, 1]
];
/** The neutral arbitrary XY curve: no adjustment anywhere. */
export const FLAT: CurvePoints = [
	[0, 0.5],
	[1, 0.5]
];

export const samePoints = (left: CurvePoints, right: CurvePoints) =>
	left.length === right.length &&
	left.every(([x, y], index) => x === right[index]![0] && y === right[index]![1]);

/**
 * The package steps a layer runs. RGB runs one `rgb` curve when all three match, otherwise one per
 * changed channel; other models run one `model-curves` step; arbitrary XY one `channel-curve`.
 */
export function packageSteps(step: LayerStep): Effect[] {
	if (step.effect !== 'curves') return [step];
	const { enabled } = step;
	if (step.model === 'xy')
		return [{ effect: 'channel-curve', enabled, x: step.x, y: step.y, points: step.points }];
	if (step.model !== 'srgb')
		// Each model's tuple follows that model's channel order, which the package names per model.
		return [
			{
				effect: 'model-curves',
				enabled,
				model: step.model,
				curves: step.curves
			} as ModelCurvesEffect
		];
	const [red, green, blue] = step.curves;
	if (samePoints(red, green) && samePoints(red, blue))
		return [{ effect: 'curves', enabled, channel: 'rgb', points: red }];
	return (['red', 'green', 'blue'] as const)
		.map((channel, index) => ({ channel, points: step.curves[index]! }))
		.filter(({ points }) => !samePoints(points, STRAIGHT))
		.map(({ channel, points }) => ({ effect: 'curves', enabled, channel, points }));
}

type CatalogEntry<K extends EffectKind> = {
	/** Name shown in menus and given to new instances. */
	readonly label: string;
	/** A neutral step: adding it leaves the image unchanged, except palette fit. */
	readonly create: () => EffectOf<K>;
};

/** Every built-in effect the website offers, in menu order. */
export const EFFECTS: { readonly [K in EffectKind]: CatalogEntry<K> } = {
	levels: {
		label: 'Levels',
		create: () => ({
			effect: 'levels',
			enabled: true,
			channel: 'rgb',
			input: { black: 0, white: 1 },
			gamma: 1,
			output: { black: 0, white: 1 }
		})
	},
	curves: {
		label: 'Curves',
		create: () => ({
			effect: 'curves',
			enabled: true,
			model: 'srgb',
			curves: [STRAIGHT, STRAIGHT, STRAIGHT]
		})
	},
	'brightness-contrast': {
		label: 'Brightness and contrast',
		create: () => ({ effect: 'brightness-contrast', enabled: true, brightness: 0, contrast: 0 })
	},
	exposure: {
		label: 'Exposure',
		create: () => ({ effect: 'exposure', enabled: true, stops: 0 })
	},
	'white-balance': {
		label: 'White balance',
		create: () => ({ effect: 'white-balance', enabled: true, temperature: 0, tint: 0 })
	},
	'hue-saturation': {
		label: 'Hue and saturation',
		create: () => ({
			effect: 'hue-saturation',
			enabled: true,
			hue: 0,
			saturation: 0,
			lightness: 0
		})
	},
	recolour: {
		label: 'Palette fit',
		create: () => ({ effect: 'recolour', enabled: true, strength: 1, recipe: null })
	}
};

/** The package accepts at most 64 steps. */
export const MAX_EFFECT_LAYERS = 64;

export const EFFECT_KINDS = Object.keys(EFFECTS) as EffectKind[];
