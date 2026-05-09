import type {
	ColorSpaceId,
	DitherSettings,
	EnabledPaletteColor,
	OutputSettings,
	SourceImageRecord
} from './types';

export type JsonValue =
	| null
	| boolean
	| number
	| string
	| readonly JsonValue[]
	| { readonly [key: string]: JsonValue | undefined };

type SourceIdentity = Omit<SourceImageRecord, 'blob'> | undefined;

type ProcessingIdentityInput = {
	output: OutputSettings;
	dither: DitherSettings;
	colorSpace: ColorSpaceId;
	paletteName: string;
	paletteSource: 'wplace' | 'custom';
	palette: readonly EnabledPaletteColor[];
	source: SourceIdentity;
};

function stableJson(value: JsonValue): string {
	if (value === null) return 'null';
	if (typeof value === 'string') return JSON.stringify(value);
	if (typeof value === 'boolean') return value ? 'true' : 'false';
	if (typeof value === 'number') {
		if (!Number.isFinite(value)) throw new Error('Settings contain a non-finite number.');
		return JSON.stringify(value);
	}
	if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`;
	const entries = Object.entries(value)
		.filter((entry): entry is [string, JsonValue] => entry[1] !== undefined)
		.sort(([left], [right]) => left.localeCompare(right));
	return `{${entries.map(([key, entry]) => `${JSON.stringify(key)}:${stableJson(entry)}`).join(',')}}`;
}

export function settingsHash(value: JsonValue): string {
	try {
		return stableJson(value);
	} catch (error) {
		throw new Error('Settings could not be hashed for processing.', { cause: error });
	}
}

function colorIdentity(color: EnabledPaletteColor): JsonValue {
	return {
		key: color.key,
		kind: color.kind,
		rgb: color.rgb
			? {
					r: color.rgb.r,
					g: color.rgb.g,
					b: color.rgb.b
				}
			: undefined
	};
}

function outputIdentity(output: OutputSettings): JsonValue {
	return {
		width: output.width,
		height: output.height,
		lockAspect: output.lockAspect,
		resize: output.resize,
		alphaMode: output.alphaMode,
		alphaThreshold: output.alphaThreshold,
		matteKey: output.matteKey,
		autoSizeOnUpload: output.autoSizeOnUpload,
		scaleFactor: output.scaleFactor,
		crop: output.crop
			? {
					x: output.crop.x,
					y: output.crop.y,
					width: output.crop.width,
					height: output.crop.height
				}
			: undefined
	};
}

export function processingIdentity(input: ProcessingIdentityInput): JsonValue {
	return {
		output: outputIdentity(input.output),
		dither: input.dither,
		colorSpace: input.colorSpace,
		palette: {
			name: input.paletteName,
			source: input.paletteSource,
			colors: input.palette.map(colorIdentity)
		},
		source: input.source
			? {
					name: input.source.name,
					width: input.source.width,
					height: input.source.height,
					type: input.source.type,
					updatedAt: input.source.updatedAt
				}
			: undefined
	};
}

export function processingIdentityHash(input: ProcessingIdentityInput): string {
	return settingsHash(processingIdentity(input));
}
