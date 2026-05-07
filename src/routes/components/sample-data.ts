/**
 * Sample data used by skeleton layouts so panels feel real before
 * processing modules exist. Not a substitute for the actual Wplace
 * palette in `src/lib/palettes/wplace.ts` (future).
 */

export type SwatchKind = 'free' | 'premium' | 'transparent' | 'custom';

export type Swatch = {
	name: string;
	hex?: string;
	kind: SwatchKind;
	enabled: boolean;
};

export const SAMPLE_PALETTE: Swatch[] = [
	{ name: 'Black', hex: '#000000', kind: 'free', enabled: true },
	{ name: 'Dark Gray', hex: '#3C3C3C', kind: 'free', enabled: true },
	{ name: 'Gray', hex: '#787878', kind: 'free', enabled: true },
	{ name: 'Light Gray', hex: '#D2D2D2', kind: 'free', enabled: true },
	{ name: 'White', hex: '#FFFFFF', kind: 'free', enabled: true },
	{ name: 'Deep Red', hex: '#600018', kind: 'free', enabled: true },
	{ name: 'Red', hex: '#ED1C24', kind: 'free', enabled: true },
	{ name: 'Orange', hex: '#FF7F27', kind: 'free', enabled: true },
	{ name: 'Gold', hex: '#F6AA09', kind: 'free', enabled: true },
	{ name: 'Yellow', hex: '#F9DD3B', kind: 'free', enabled: true },
	{ name: 'Pale Yellow', hex: '#FFFABC', kind: 'free', enabled: true },
	{ name: 'Green', hex: '#0EB968', kind: 'free', enabled: true },
	{ name: 'Light Green', hex: '#13E67B', kind: 'free', enabled: true },
	{ name: 'Mint', hex: '#87FF5E', kind: 'free', enabled: true },
	{ name: 'Teal', hex: '#0C816E', kind: 'free', enabled: true },
	{ name: 'Cyan', hex: '#13E1BE', kind: 'free', enabled: true },
	{ name: 'Sky', hex: '#60F7F2', kind: 'free', enabled: true },
	{ name: 'Blue', hex: '#28509E', kind: 'free', enabled: true },
	{ name: 'Light Blue', hex: '#4093E4', kind: 'free', enabled: true },
	{ name: 'Indigo', hex: '#6B50F6', kind: 'free', enabled: true },
	{ name: 'Lavender', hex: '#99B1FB', kind: 'free', enabled: true },
	{ name: 'Purple', hex: '#780C99', kind: 'free', enabled: true },
	{ name: 'Magenta', hex: '#AA38B9', kind: 'free', enabled: true },
	{ name: 'Pink', hex: '#E09FF9', kind: 'free', enabled: true },
	{ name: 'Hot Pink', hex: '#CB007A', kind: 'free', enabled: true },
	{ name: 'Rose', hex: '#EC1F80', kind: 'free', enabled: true },
	{ name: 'Salmon', hex: '#F38DA9', kind: 'free', enabled: true },
	{ name: 'Brown', hex: '#684634', kind: 'free', enabled: true },
	{ name: 'Tan', hex: '#95682A', kind: 'free', enabled: true },
	{ name: 'Peach', hex: '#F8B277', kind: 'free', enabled: true },
	{ name: 'Silver', hex: '#AAAAAA', kind: 'premium', enabled: false },
	{ name: 'Crimson', hex: '#A50E1E', kind: 'premium', enabled: false },
	{ name: 'Coral', hex: '#FA8072', kind: 'premium', enabled: false },
	// Two custom entries so the grid swatches demonstrate the editable
	// state — edit/delete enabled, full-opacity icons on hover/focus.
	{ name: 'Studio Pink', hex: '#FF5C8A', kind: 'custom', enabled: true },
	{ name: 'Studio Teal', hex: '#1FB6A3', kind: 'custom', enabled: true },
	{ name: 'Transparent', kind: 'transparent', enabled: true }
];

export type ColorSpaceOption = {
	id: string;
	label: string;
	short: string;
	math: string;
};

export const COLOR_SPACES: ColorSpaceOption[] = [
	{
		id: 'oklab',
		label: 'OKLab',
		short:
			'Modern perceptual space; Euclidean distance over (L, a, b) approximates perceived difference well.',
		math: 'd² = (L₁−L₂)² + (a₁−a₂)² + (b₁−b₂)²'
	},
	{
		id: 'srgb',
		label: 'sRGB',
		short: 'Browser display space. Fast but not perceptually uniform.',
		math: 'd² = (R₁−R₂)² + (G₁−G₂)² + (B₁−B₂)²'
	},
	{
		id: 'linear-rgb',
		label: 'Linear RGB',
		short: 'Removes sRGB gamma before distance math; better for light mixing.',
		math: 'd² over linearized channels'
	},
	{
		id: 'weighted-rgb',
		label: 'Weighted RGB',
		short: 'CompuPhase weighted RGB. Cheap channel-weighted approximation of perceived difference.',
		math: '(2+r̄/256)·ΔR² + 4·ΔG² + (2+(255−r̄)/256)·ΔB²'
	},
	{
		id: 'weighted-rgb-601',
		label: 'Weighted RGB · Rec.601',
		short: 'Legacy luma weights emphasize green, then red, then blue.',
		math: '0.299·ΔR² + 0.587·ΔG² + 0.114·ΔB²'
	},
	{
		id: 'weighted-rgb-709',
		label: 'Weighted RGB · Rec.709',
		short: 'HDTV luma weights with stronger green emphasis.',
		math: '0.2126·ΔR² + 0.7152·ΔG² + 0.0722·ΔB²'
	},
	{
		id: 'cielab',
		label: 'CIELAB ΔE76',
		short: 'Approximates human color difference via XYZ → CIELAB conversion.',
		math: 'ΔE*ab = √((ΔL)² + (Δa)² + (Δb)²)'
	},
	{
		id: 'oklch',
		label: 'OKLCH',
		short: 'Cylindrical OKLab. Hue distance wraps around the color wheel.',
		math: 'd uses ΔL, ΔC, and circular Δh'
	}
];

export type DitherMethod = 'none' | 'threshold' | 'error-diffusion';
export type DitherField = 'none' | 'ordered' | 'noise' | 'kernel';

export type DitherOption = {
	id: string;
	label: string;
	family: 'none' | 'ordered' | 'error-diffusion' | 'noise';
	method: DitherMethod;
	field: DitherField;
	sku: string;
	short: string;
	math: string;
};

export const DITHER_ALGORITHMS: DitherOption[] = [
	{
		id: 'none',
		label: 'None',
		family: 'none',
		method: 'none',
		field: 'none',
		sku: 'direct.none',
		short: 'Direct nearest-color quantization. Fast; can flatten gradients.',
		math: 'index = nearestPaletteColor(pixel)'
	},
	{
		id: 'bayer-2',
		label: 'Bayer 2×2',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-2',
		short: 'Smallest ordered threshold matrix; strongest visible repetition.',
		math: 'pixel += (Bayer₂[x mod 2,y mod 2] − 0.5) · strength'
	},
	{
		id: 'bayer-4',
		label: 'Bayer 4×4',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-4',
		short: 'Ordered threshold matrix. Crisp, repeating pattern.',
		math: 'pixel += (Bayer₄[x mod 4,y mod 4] − 0.5) · strength'
	},
	{
		id: 'bayer-8',
		label: 'Bayer 8×8',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-8',
		short: 'Larger matrix; less obvious repetition than 4×4.',
		math: 'pixel += (Bayer₈[x mod 8,y mod 8] − 0.5) · strength'
	},
	{
		id: 'bayer-16',
		label: 'Bayer 16×16',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-16',
		short: 'Largest ordered matrix; smoothest of the Bayer family.',
		math: 'pixel += (Bayer₁₆[x mod 16,y mod 16] − 0.5) · strength'
	},
	{
		id: 'floyd-steinberg',
		label: 'Floyd–Steinberg',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.floyd-steinberg',
		short: 'Distributes quantization error to four future neighbors (7/16, 3/16, 5/16, 1/16).',
		math: 'error = pixel − quantized; diffuse {→7, ↙3, ↓5, ↘1}/16'
	},
	{
		id: 'sierra',
		label: 'Sierra',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.sierra',
		short: 'Wider three-row error diffusion kernel.',
		math: 'error = pixel − quantized; diffuse Sierra weights /32 across three rows'
	},
	{
		id: 'sierra-lite',
		label: 'Sierra Lite',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.sierra-lite',
		short: 'Cheaper Sierra variant; smaller neighborhood.',
		math: 'error = pixel − quantized; diffuse {→2, ↙1, ↓1}/4'
	},
	{
		id: 'random',
		label: 'Random',
		family: 'noise',
		method: 'threshold',
		field: 'noise',
		sku: 'threshold.noise.white',
		short: 'Seeded per-pixel noise perturbs the quantization threshold.',
		math: 'pixel += (mulberry32(seed,x,y) − 0.5) · strength'
	}
];

export type ResizeOption = { id: string; label: string };

export const RESIZE_MODES: ResizeOption[] = [
	{ id: 'lanczos3', label: 'Lanczos3' },
	{ id: 'bilinear', label: 'Bilinear' },
	{ id: 'nearest', label: 'Nearest' },
	{ id: 'area', label: 'Area / Box' }
];

export const ALPHA_MODES: ResizeOption[] = [
	{ id: 'preserve', label: 'Preserve transparency' },
	{ id: 'premultiplied', label: 'Premultiplied' },
	{ id: 'matte', label: 'Matte' }
];

export const PLACEMENT_MODES: ResizeOption[] = [
	{ id: 'everywhere', label: 'Everywhere' },
	{ id: 'adaptive', label: 'Adaptive' }
];
