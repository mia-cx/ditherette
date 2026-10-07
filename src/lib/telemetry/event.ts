import { Schema } from 'effect';
import type { Effect, FitLook, FitMeasurements, PaletteFitSpace } from 'ditherette';
import { paletteColorEnabled } from '$lib/palette/wplace';
import type { ColorSpaceId, DitherSettings, OutputSettings, Palette } from '$lib/processing/types';

const Colour = Schema.Struct({
	rgb: Schema.NullOr(Schema.Tuple([Schema.Number, Schema.Number, Schema.Number])),
	enabled: Schema.Boolean
});

/** The fit's measured internals as the package reports them. */
const Measurements = Schema.Struct({
	toneQuantiles: Schema.Tuple([
		Schema.Number,
		Schema.Number,
		Schema.Number,
		Schema.Number,
		Schema.Number
	]),
	greyShift: Schema.Number,
	reach: Schema.Array(Schema.Array(Schema.Number))
});

const Fit = Schema.Struct({
	/** The step's position in `settings.effects`. */
	step: Schema.Number,
	look: Schema.Literals(['natural', 'fitted', 'vivid']),
	space: Schema.Literals(['oklab', 'cielab']),
	edited: Schema.Boolean,
	looksApplied: Schema.Array(Schema.Literals(['natural', 'fitted', 'vivid'])),
	measurements: Schema.NullOr(Measurements)
});

/** The anonymous v1 export event: settings and colour measurements, never the image. */
export const ExportEvent = Schema.Struct({
	version: Schema.Literal(1),
	settings: Schema.Struct({
		output: Schema.StructWithRest(Schema.Struct({ cropped: Schema.Boolean }), [
			Schema.Record(Schema.String, Schema.Unknown)
		]),
		dither: Schema.Record(Schema.String, Schema.Unknown),
		colorSpace: Schema.String,
		palette: Schema.Struct({
			name: Schema.String,
			colours: Schema.Array(Colour)
		}),
		effects: Schema.Array(Schema.Unknown)
	}),
	fits: Schema.Array(Fit),
	export: Schema.Struct({
		width: Schema.Number,
		height: Schema.Number,
		format: Schema.Literal('png')
	})
});

export type ExportEvent = typeof ExportEvent.Type;

/** One palette-fit step's contribution to the event. */
export interface FitEvent {
	readonly step: number;
	readonly look: FitLook;
	readonly space: PaletteFitSpace;
	readonly edited: boolean;
	readonly looksApplied: readonly FitLook[];
	readonly measurements: FitMeasurements | null;
}

/** The pieces of the payload assembled from the stores at export time. */
export interface ExportPayloadInput {
	readonly output: OutputSettings;
	readonly dither: DitherSettings;
	readonly colorSpace: ColorSpaceId;
	readonly palette: Palette;
	/** Every colour's enabled flag, by colour key. */
	readonly enabled: Readonly<Record<string, boolean>>;
	readonly effects: readonly Effect[];
	readonly fits: readonly FitEvent[];
	readonly width: number;
	readonly height: number;
}

/**
 * Build the v1 event. Crop coordinates are source metadata, so `output` carries only the
 * boolean `cropped`; everything else is settings and colour statistics.
 */
export function buildExportEvent(input: ExportPayloadInput): ExportEvent {
	// Crop is source metadata; the event keeps only the flag.
	const { crop, ...rest } = input.output;
	const output = { ...rest, cropped: crop !== undefined };
	return {
		version: 1,
		settings: {
			output,
			dither: input.dither,
			colorSpace: input.colorSpace,
			palette: {
				name: input.palette.name,
				colours: input.palette.colors.map((colour) => ({
					rgb: colour.rgb ? [colour.rgb.r, colour.rgb.g, colour.rgb.b] : null,
					enabled: paletteColorEnabled(input.enabled, input.palette.name, colour.key)
				}))
			},
			effects: [...input.effects]
		},
		fits: input.fits.map((fit) => ({ ...fit })),
		export: { width: input.width, height: input.height, format: 'png' }
	};
}
