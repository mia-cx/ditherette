import type { EnabledPaletteColor, Palette, PaletteColor } from '$lib/processing/types';

const free = [
	['Black', '#000000'],
	['Dark Gray', '#3C3C3C'],
	['Gray', '#787878'],
	['Light Gray', '#D2D2D2'],
	['White', '#FFFFFF'],
	['Deep Red', '#600018'],
	['Red', '#ED1C24'],
	['Orange', '#FF7F27'],
	['Gold', '#F6AA09'],
	['Yellow', '#F9DD3B'],
	['Pale Yellow', '#FFFABC'],
	['Green', '#0EB968'],
	['Light Green', '#13E67B'],
	['Mint', '#87FF5E'],
	['Deep Teal', '#0C816E'],
	['Teal', '#10AE82'],
	['Cyan', '#13E1BE'],
	['Aqua', '#60F7F2'],
	['Blue', '#28509E'],
	['Light Blue', '#4093E4'],
	['Indigo', '#6B50F6'],
	['Periwinkle', '#99B1FB'],
	['Purple', '#780C99'],
	['Magenta', '#AA38B9'],
	['Lavender', '#E09FF9'],
	['Hot Pink', '#CB007A'],
	['Rose', '#EC1F80'],
	['Pink', '#F38DA9'],
	['Brown', '#684634'],
	['Tan', '#95682A'],
	['Peach', '#F8B277']
] as const;

const premium = [
	['Silver', '#AAAAAA'],
	['Crimson', '#A50E1E'],
	['Coral', '#FA8072'],
	['Burnt Orange', '#E45C1A'],
	['Olive', '#9C8431'],
	['Mustard', '#C5AD31'],
	['Sand', '#E8D45F'],
	['Moss', '#4A6B3A'],
	['Leaf', '#5A944A'],
	['Pistachio', '#84C573'],
	['Ocean', '#0F799F'],
	['Pale Aqua', '#BBFAF2'],
	['Sky Blue', '#7DC7FF'],
	['Violet', '#4D31B8'],
	['Slate Purple', '#4A4284'],
	['Soft Purple', '#7A71C4'],
	['Pale Violet', '#B5AEF1'],
	['Brick', '#9B5249'],
	['Dusty Rose', '#D18078'],
	['Light Coral', '#FAB6A4'],
	['Caramel', '#DBA463'],
	['Umber', '#7B6352'],
	['Taupe', '#9C846B'],
	['Beige', '#D6B594'],
	['Terracotta', '#D18051'],
	['Apricot', '#FFC5A5'],
	['Army', '#6D643F'],
	['Khaki', '#948C6B'],
	['Cream', '#CDC59E'],
	['Charcoal Blue', '#333941'],
	['Steel', '#6D758D'],
	['Pale Steel', '#B3B9D1']
] as const;

function hexToRgb(hex: string) {
	return {
		r: Number.parseInt(hex.slice(1, 3), 16),
		g: Number.parseInt(hex.slice(3, 5), 16),
		b: Number.parseInt(hex.slice(5, 7), 16)
	};
}

function visibleColor(
	[name, hex]: readonly [string, string],
	kind: 'free' | 'premium'
): PaletteColor {
	return { name, key: hex.toUpperCase(), rgb: hexToRgb(hex), kind };
}

export const TRANSPARENT_KEY = 'transparent';
export const WPLACE_PALETTE_NAME = 'Wplace (Default)';

export const WPLACE_PALETTE: PaletteColor[] = [
	...free.map((color) => visibleColor(color, 'free')),
	...premium.map((color) => visibleColor(color, 'premium')),
	{ name: 'Transparent', key: TRANSPARENT_KEY, kind: 'transparent' }
];

export const WPLACE: Palette = {
	name: WPLACE_PALETTE_NAME,
	source: 'wplace',
	colors: WPLACE_PALETTE
};

export function paletteEnabledKey(paletteName: string, colorKey: string) {
	return paletteName === WPLACE_PALETTE_NAME ? colorKey : `${paletteName}\u0000${colorKey}`;
}

export function defaultEnabledState(): Record<string, boolean> {
	return Object.fromEntries(
		WPLACE_PALETTE.map((color) => [paletteEnabledKey(WPLACE_PALETTE_NAME, color.key), true])
	);
}

export function enabledPalette(
	paletteOrEnabled: Palette | Record<string, boolean>,
	enabledMaybe?: Record<string, boolean>
): EnabledPaletteColor[] {
	const palette = enabledMaybe ? (paletteOrEnabled as Palette) : WPLACE;
	const enabled = enabledMaybe ?? (paletteOrEnabled as Record<string, boolean>);
	return palette.colors
		.filter((color) => enabled[paletteEnabledKey(palette.name, color.key)] !== false)
		.map((color) => ({
			...color,
			enabled: true
		}));
}

export function visibleEnabledColors(enabled: Record<string, boolean>): EnabledPaletteColor[] {
	return enabledPalette(enabled).filter((color) => color.kind !== 'transparent' && color.rgb);
}

export function darkestVisible(colors: EnabledPaletteColor[]): EnabledPaletteColor | undefined {
	return colors.reduce<EnabledPaletteColor | undefined>((darkest, color) => {
		if (!color.rgb) return darkest;
		if (!darkest?.rgb) return color;
		const sum = color.rgb.r + color.rgb.g + color.rgb.b;
		const darkestSum = darkest.rgb.r + darkest.rgb.g + darkest.rgb.b;
		return sum < darkestSum ? color : darkest;
	}, undefined);
}

export function paletteSummary(enabled: Record<string, boolean>): string {
	const active = enabledPalette(enabled).length;
	return `${active} / ${WPLACE_PALETTE.length} colors active`;
}
