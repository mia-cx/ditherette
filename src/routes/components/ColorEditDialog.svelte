<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogFooter,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Select, SelectContent, SelectItem, SelectTrigger } from '$lib/components/ui/select';
	import ColorPlanePicker from './ColorPlanePicker.svelte';
	import ColorSliderPicker from './ColorSliderPicker.svelte';
	import ColorSpacePreviewPicker from './ColorSpacePreviewPicker.svelte';
	import ColorWheelPicker from './ColorWheelPicker.svelte';

	type PickerMode =
		| 'hsl-wheel'
		| 'hue'
		| 'saturation'
		| 'lightness'
		| 'rgb-sliders'
		| 'hsl-sliders'
		| 'oklab'
		| 'oklch';
	type Rgb = { r: number; g: number; b: number };
	type Hsl = { h: number; s: number; l: number };
	type Hsv = { h: number; s: number; v: number };
	type Oklab = { l: number; a: number; b: number };
	type Oklch = { l: number; c: number; h: number };
	type Point = { x: number; y: number };
	type SliderChannel = {
		label: string;
		value: number;
		min: number;
		max: number;
		step?: number;
		background: string;
		onChange: (value: number) => void;
	};
	type ColorSyncOptions = {
		preserveHsv?: Hsv;
		preserveHsl?: Hsl;
		preserveOklch?: Oklch;
	};
	type Props = {
		open: boolean;
		mode: 'add' | 'edit' | 'duplicate';
		name: string;
		hex: string;
		tags: string[];
		onSave: () => void;
	};

	const pickerOptions: { id: PickerMode; label: string }[] = [
		{ id: 'hsl-wheel', label: 'HSL Colour Wheel' },
		{ id: 'hue', label: 'Hue' },
		{ id: 'saturation', label: 'Saturation' },
		{ id: 'lightness', label: 'Lightness' },
		{ id: 'rgb-sliders', label: 'RGB Sliders' },
		{ id: 'hsl-sliders', label: 'HSL Sliders' },
		{ id: 'oklab', label: 'OKLab' },
		{ id: 'oklch', label: 'OKLCH' }
	];

	let {
		open = $bindable(),
		mode,
		name = $bindable(),
		hex = $bindable(),
		tags = $bindable(),
		onSave
	}: Props = $props();

	let picker = $state<PickerMode>('hsl-wheel');
	let tagDraft = $state('');
	let hsv = $state<Hsv>({ h: 0, s: 100, v: 100 });
	let hsl = $state<Hsl>({ h: 0, s: 100, l: 50 });
	let oklab = $state<Oklab>({ l: 0.7, a: 0, b: 0 });
	let oklch = $state<Oklch>({ l: 0.7, c: 0, h: 0 });
	let syncingFromPicker = false;

	const triangleRadiusRatio = 0.25;
	const rgb = $derived(rgbFromHex(hex) ?? hsvToRgb(hsv));
	const hueRailBackground =
		'linear-gradient(to right, #ff0000 0%, #ffff00 16.666%, #00ff00 33.333%, #00ffff 50%, #0000ff 66.666%, #ff00ff 83.333%, #ff0000 100%)';
	const hueWheelBackground =
		'conic-gradient(from 90deg, #ff0000 0deg, #ffff00 60deg, #00ff00 120deg, #00ffff 180deg, #0000ff 240deg, #ff00ff 300deg, #ff0000 360deg)';
	const huePlaneBackground = $derived(
		`linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${hsv.h} 100% 50%))`
	);
	const saturationPlaneBackground = $derived(
		`linear-gradient(to bottom, #fff, transparent 50%, #000), linear-gradient(to right, hsl(0 ${hsl.s}% 50%), hsl(60 ${hsl.s}% 50%), hsl(120 ${hsl.s}% 50%), hsl(180 ${hsl.s}% 50%), hsl(240 ${hsl.s}% 50%), hsl(300 ${hsl.s}% 50%), hsl(360 ${hsl.s}% 50%))`
	);
	const lightnessPlaneBackground = $derived(
		`linear-gradient(to bottom, transparent, hsl(0 0% ${hsl.l}%)), linear-gradient(to right, hsl(0 100% ${hsl.l}%), hsl(60 100% ${hsl.l}%), hsl(120 100% ${hsl.l}%), hsl(180 100% ${hsl.l}%), hsl(240 100% ${hsl.l}%), hsl(300 100% ${hsl.l}%), hsl(360 100% ${hsl.l}%))`
	);
	const planeBackground = $derived.by(() => {
		if (picker === 'saturation') return saturationPlaneBackground;
		if (picker === 'lightness') return lightnessPlaneBackground;
		return huePlaneBackground;
	});
	const planeHandleStyle = $derived.by(() => {
		if (picker === 'saturation') return `left: ${(hsl.h / 360) * 100}%; top: ${100 - hsl.l}%;`;
		if (picker === 'lightness') return `left: ${(hsl.h / 360) * 100}%; top: ${100 - hsl.s}%;`;
		return `left: ${hsv.s}%; top: ${100 - hsv.v}%;`;
	});
	const triangleHandleStyle = $derived.by(() => {
		const point = trianglePointForHsv(hsv);
		return `left: ${point.x}%; top: ${point.y}%;`;
	});
	const hueWheelHandleStyle = $derived.by(() => {
		const point = wheelPointForHue(hsv.h);
		return `left: ${point.x}%; top: ${point.y}%;`;
	});
	const sliderChannels = $derived.by(sliderChannelsForMode);
	const oklabValues = $derived([
		{ label: 'L', value: oklab.l.toFixed(3) },
		{ label: 'a', value: oklab.a.toFixed(3) },
		{ label: 'b', value: oklab.b.toFixed(3) }
	]);
	const oklchValues = $derived([
		{ label: 'L', value: oklch.l.toFixed(3) },
		{ label: 'C', value: oklch.c.toFixed(3) },
		{ label: 'h', value: `${oklch.h.toFixed(0)}°` }
	]);

	$effect(() => {
		if (syncingFromPicker) return;
		const nextRgb = rgbFromHex(hex);
		if (!nextRgb) return;
		syncColorModels(nextRgb);
	});

	function setColorFromRgb(next: Rgb, options: ColorSyncOptions = {}) {
		const clamped = {
			r: clamp(next.r, 0, 255),
			g: clamp(next.g, 0, 255),
			b: clamp(next.b, 0, 255)
		};
		syncingFromPicker = true;
		hex = hexFromRgb(clamped);
		syncColorModels(clamped, options);
		queueMicrotask(() => (syncingFromPicker = false));
	}

	function syncColorModels(color: Rgb, options: ColorSyncOptions = {}) {
		const nextOklab = rgbToOklab(color);
		hsv = options.preserveHsv ?? rgbToHsv(color);
		hsl = options.preserveHsl ?? rgbToHsl(color);
		oklab = nextOklab;
		oklch = options.preserveOklch ?? oklabToOklch(nextOklab);
	}

	function updateHsv(patch: Partial<Hsv>) {
		const next = { ...hsv, ...patch };
		setColorFromRgb(hsvToRgb(next), { preserveHsv: next });
	}

	function updateHsl(patch: Partial<Hsl>) {
		const next = { ...hsl, ...patch };
		setColorFromRgb(hslToRgb(next), { preserveHsl: next });
	}

	function updateOklab(patch: Partial<Oklab>) {
		const next = { ...oklab, ...patch };
		setColorFromRgb(oklabToRgb(next));
	}

	function updateOklch(patch: Partial<Oklch>) {
		const next = { ...oklch, ...patch };
		setColorFromRgb(oklabToRgb(oklchToOklab(next)), { preserveOklch: next });
	}

	function sliderChannelsForMode(): SliderChannel[] {
		if (picker === 'rgb-sliders') {
			return [
				{
					label: 'R',
					value: rgb.r,
					min: 0,
					max: 255,
					background: `linear-gradient(to right, rgb(0 ${rgb.g} ${rgb.b}), rgb(255 ${rgb.g} ${rgb.b}))`,
					onChange: (value) => setColorFromRgb({ ...rgb, r: value })
				},
				{
					label: 'G',
					value: rgb.g,
					min: 0,
					max: 255,
					background: `linear-gradient(to right, rgb(${rgb.r} 0 ${rgb.b}), rgb(${rgb.r} 255 ${rgb.b}))`,
					onChange: (value) => setColorFromRgb({ ...rgb, g: value })
				},
				{
					label: 'B',
					value: rgb.b,
					min: 0,
					max: 255,
					background: `linear-gradient(to right, rgb(${rgb.r} ${rgb.g} 0), rgb(${rgb.r} ${rgb.g} 255))`,
					onChange: (value) => setColorFromRgb({ ...rgb, b: value })
				}
			];
		}
		if (picker === 'hsl-sliders') {
			return [
				{
					label: 'H',
					value: hsl.h,
					min: 0,
					max: 360,
					background: hueRailBackground,
					onChange: (value) => updateHsl({ h: value })
				},
				{
					label: 'S',
					value: hsl.s,
					min: 0,
					max: 100,
					background: `linear-gradient(to right, hsl(${hsl.h} 0% ${hsl.l}%), hsl(${hsl.h} 100% ${hsl.l}%))`,
					onChange: (value) => updateHsl({ s: value })
				},
				{
					label: 'L',
					value: hsl.l,
					min: 0,
					max: 100,
					background: `linear-gradient(to right, #000, hsl(${hsl.h} ${hsl.s}% 50%), #fff)`,
					onChange: (value) => updateHsl({ l: value })
				}
			];
		}
		if (picker === 'oklab') {
			return [
				{
					label: 'L',
					value: oklab.l,
					min: 0,
					max: 1,
					step: 0.001,
					background: 'linear-gradient(to right, #000, #fff)',
					onChange: (value) => updateOklab({ l: value })
				},
				{
					label: 'a',
					value: oklab.a,
					min: -0.4,
					max: 0.4,
					step: 0.001,
					background: 'linear-gradient(to right, #00a676, #777, #ff4b8b)',
					onChange: (value) => updateOklab({ a: value })
				},
				{
					label: 'b',
					value: oklab.b,
					min: -0.4,
					max: 0.4,
					step: 0.001,
					background: 'linear-gradient(to right, #4f80ff, #777, #ffe45c)',
					onChange: (value) => updateOklab({ b: value })
				}
			];
		}
		if (picker === 'oklch') {
			return [
				{
					label: 'L',
					value: oklch.l,
					min: 0,
					max: 1,
					step: 0.001,
					background: 'linear-gradient(to right, #000, #fff)',
					onChange: (value) => updateOklch({ l: value })
				},
				{
					label: 'C',
					value: oklch.c,
					min: 0,
					max: 0.4,
					step: 0.001,
					background: `linear-gradient(to right, oklch(${oklch.l} 0 ${oklch.h}), oklch(${oklch.l} 0.4 ${oklch.h}))`,
					onChange: (value) => updateOklch({ c: value })
				},
				{
					label: 'h',
					value: oklch.h,
					min: 0,
					max: 360,
					background: hueRailBackground,
					onChange: (value) => updateOklch({ h: value })
				}
			];
		}
		if (picker === 'hue')
			return [
				{
					label: 'H',
					value: hsv.h,
					min: 0,
					max: 360,
					background: hueRailBackground,
					onChange: (value) => updateHsv({ h: value })
				}
			];
		if (picker === 'saturation')
			return [
				{
					label: 'S',
					value: hsl.s,
					min: 0,
					max: 100,
					background: `linear-gradient(to right, hsl(${hsl.h} 0% ${hsl.l}%), hsl(${hsl.h} 100% ${hsl.l}%))`,
					onChange: (value) => updateHsl({ s: value })
				}
			];
		if (picker === 'lightness')
			return [
				{
					label: 'L',
					value: hsl.l,
					min: 0,
					max: 100,
					background: `linear-gradient(to right, #000, hsl(${hsl.h} ${hsl.s}% 50%), #fff)`,
					onChange: (value) => updateHsl({ l: value })
				}
			];
		return [];
	}

	function pickFromPlane(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const x = clamp((next.clientX - rect.left) / rect.width, 0, 1);
			const y = clamp((next.clientY - rect.top) / rect.height, 0, 1);
			if (picker === 'saturation') {
				updateHsl({ h: x * 360, l: (1 - y) * 100 });
				return;
			}
			if (picker === 'lightness') {
				updateHsl({ h: x * 360, s: (1 - y) * 100 });
				return;
			}
			updateHsv({ s: x * 100, v: (1 - y) * 100 });
		};
		captureDrag(event, target, update);
	}

	function pickFromWheel(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const dragMode = wheelDragMode(event, target);
		const update = (next: PointerEvent) => {
			const point = wheelPoint(next, target);
			if (dragMode === 'hue') {
				updateHsv({ h: hueFromWheelPoint(point.x, point.y) });
				return;
			}
			updateHsv(hsvFromTrianglePoint(point.x, point.y, target.getBoundingClientRect().width));
		};
		captureDrag(event, target, update);
	}

	function captureDrag(
		event: PointerEvent,
		target: HTMLElement,
		update: (event: PointerEvent) => void
	) {
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => (target.onpointermove = null);
	}

	function wheelDragMode(event: PointerEvent, target: HTMLElement): 'hue' | 'triangle' {
		const point = wheelPoint(event, target);
		const outer = target.getBoundingClientRect().width / 2;
		return Math.hypot(point.x, point.y) >= outer * 0.74 ? 'hue' : 'triangle';
	}

	function wheelPoint(event: PointerEvent, target: HTMLElement) {
		const rect = target.getBoundingClientRect();
		return {
			x: event.clientX - (rect.left + rect.width / 2),
			y: event.clientY - (rect.top + rect.height / 2)
		};
	}

	function hueFromWheelPoint(x: number, y: number) {
		return ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
	}

	function wheelPointForHue(hue: number) {
		const angle = (hue * Math.PI) / 180;
		return { x: 50 + Math.cos(angle) * 43, y: 50 + Math.sin(angle) * 43 };
	}

	function triangleVertices(width: number) {
		const radius = width * triangleRadiusRatio;
		return {
			white: { x: -radius / 2, y: -(Math.sqrt(3) / 2) * radius },
			black: { x: -radius / 2, y: (Math.sqrt(3) / 2) * radius },
			hue: { x: radius, y: 0 }
		};
	}

	function hsvFromTrianglePoint(x: number, y: number, width: number): Partial<Hsv> {
		const vertices = triangleVertices(width);
		const local = rotatePoint({ x, y }, -hsv.h);
		const point = closestPointInTriangle(local, vertices.white, vertices.black, vertices.hue);
		const weights = barycentric(point, vertices.white, vertices.black, vertices.hue);
		const value = clamp((1 - weights.black) * 100, 0, 100);
		const saturation =
			value === 0 ? 0 : clamp((weights.hue / (weights.white + weights.hue)) * 100, 0, 100);
		return { s: saturation, v: value };
	}

	function trianglePointForHsv(color: Hsv) {
		const value = color.v / 100;
		const saturation = color.s / 100;
		const whiteWeight = value * (1 - saturation);
		const blackWeight = 1 - value;
		const hueWeight = value * saturation;
		const radius = triangleRadiusRatio * 100;
		const local = {
			x: whiteWeight * (-radius / 2) + blackWeight * (-radius / 2) + hueWeight * radius,
			y: whiteWeight * (-(Math.sqrt(3) / 2) * radius) + blackWeight * ((Math.sqrt(3) / 2) * radius)
		};
		const point = rotatePoint(local, color.h);
		return { x: 50 + point.x, y: 50 + point.y };
	}

	function barycentric(point: Point, white: Point, black: Point, hue: Point) {
		const denom = (black.y - hue.y) * (white.x - hue.x) + (hue.x - black.x) * (white.y - hue.y);
		const whiteWeight =
			((black.y - hue.y) * (point.x - hue.x) + (hue.x - black.x) * (point.y - hue.y)) / denom;
		const blackWeight =
			((hue.y - white.y) * (point.x - hue.x) + (white.x - hue.x) * (point.y - hue.y)) / denom;
		return { white: whiteWeight, black: blackWeight, hue: 1 - whiteWeight - blackWeight };
	}

	function closestPointInTriangle(point: Point, white: Point, black: Point, hue: Point) {
		const weights = barycentric(point, white, black, hue);
		if (weights.white >= 0 && weights.black >= 0 && weights.hue >= 0) return point;
		return [
			closestPointOnSegment(point, white, black),
			closestPointOnSegment(point, black, hue),
			closestPointOnSegment(point, hue, white)
		].reduce((best, candidate) =>
			distanceSquared(point, candidate) < distanceSquared(point, best) ? candidate : best
		);
	}

	function closestPointOnSegment(point: Point, start: Point, end: Point) {
		const dx = end.x - start.x;
		const dy = end.y - start.y;
		const lengthSquared = dx * dx + dy * dy;
		const t =
			lengthSquared === 0
				? 0
				: clamp(((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared, 0, 1);
		return { x: start.x + t * dx, y: start.y + t * dy };
	}

	function rotatePoint(point: Point, degrees: number) {
		const radians = (degrees * Math.PI) / 180;
		const cos = Math.cos(radians);
		const sin = Math.sin(radians);
		return { x: point.x * cos - point.y * sin, y: point.x * sin + point.y * cos };
	}

	function distanceSquared(left: Point, right: Point) {
		return (left.x - right.x) ** 2 + (left.y - right.y) ** 2;
	}

	function addTag() {
		const next = tagDraft.trim();
		if (!next || tags.includes(next)) return;
		tags = [...tags, next];
		tagDraft = '';
	}

	function removeTag(tag: string) {
		tags = tags.filter((item) => item !== tag);
	}

	function title() {
		if (mode === 'edit') return 'Edit color';
		if (mode === 'duplicate') return 'Duplicate color';
		return 'Add color';
	}

	function clamp(value: number, min: number, max: number) {
		return Math.min(max, Math.max(min, value));
	}

	function componentToHex(value: number) {
		return Math.round(clamp(value, 0, 255))
			.toString(16)
			.padStart(2, '0')
			.toUpperCase();
	}

	function hexFromRgb(color: Rgb) {
		return `#${componentToHex(color.r)}${componentToHex(color.g)}${componentToHex(color.b)}`;
	}

	function rgbFromHex(value: string): Rgb | undefined {
		const expanded = /^#[0-9a-fA-F]{3}$/.test(value)
			? `#${value[1]}${value[1]}${value[2]}${value[2]}${value[3]}${value[3]}`
			: value;
		if (!/^#[0-9a-fA-F]{6}$/.test(expanded)) return undefined;
		return {
			r: Number.parseInt(expanded.slice(1, 3), 16),
			g: Number.parseInt(expanded.slice(3, 5), 16),
			b: Number.parseInt(expanded.slice(5, 7), 16)
		};
	}

	function rgbToHsv(color: Rgb): Hsv {
		const r = color.r / 255;
		const g = color.g / 255;
		const b = color.b / 255;
		const max = Math.max(r, g, b);
		const min = Math.min(r, g, b);
		const d = max - min;
		let h = 0;
		if (d !== 0) {
			if (max === r) h = ((g - b) / d) % 6;
			else if (max === g) h = (b - r) / d + 2;
			else h = (r - g) / d + 4;
		}
		return { h: (h * 60 + 360) % 360, s: max === 0 ? 0 : (d / max) * 100, v: max * 100 };
	}

	function hsvToRgb({ h, s, v }: Hsv): Rgb {
		s /= 100;
		v /= 100;
		const c = v * s;
		const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
		const m = v - c;
		const [rp, gp, bp] = hueSegment(h, c, x);
		return { r: (rp + m) * 255, g: (gp + m) * 255, b: (bp + m) * 255 };
	}

	function rgbToHsl(color: Rgb): Hsl {
		const r = color.r / 255;
		const g = color.g / 255;
		const b = color.b / 255;
		const max = Math.max(r, g, b);
		const min = Math.min(r, g, b);
		const l = (max + min) / 2;
		const d = max - min;
		if (d === 0) return { h: 0, s: 0, l: l * 100 };
		const s = d / (1 - Math.abs(2 * l - 1));
		let h: number;
		if (max === r) h = ((g - b) / d) % 6;
		else if (max === g) h = (b - r) / d + 2;
		else h = (r - g) / d + 4;
		return { h: (h * 60 + 360) % 360, s: s * 100, l: l * 100 };
	}

	function hslToRgb({ h, s, l }: Hsl): Rgb {
		s /= 100;
		l /= 100;
		const c = (1 - Math.abs(2 * l - 1)) * s;
		const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
		const m = l - c / 2;
		const [rp, gp, bp] = hueSegment(h, c, x);
		return { r: (rp + m) * 255, g: (gp + m) * 255, b: (bp + m) * 255 };
	}

	function hueSegment(h: number, c: number, x: number) {
		return h < 60
			? [c, x, 0]
			: h < 120
				? [x, c, 0]
				: h < 180
					? [0, c, x]
					: h < 240
						? [0, x, c]
						: h < 300
							? [x, 0, c]
							: [c, 0, x];
	}

	function srgbToLinear(value: number) {
		const channel = value / 255;
		return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
	}

	function linearToSrgb(value: number) {
		const channel = value <= 0.0031308 ? 12.92 * value : 1.055 * value ** (1 / 2.4) - 0.055;
		return clamp(channel * 255, 0, 255);
	}

	function rgbToOklab(color: Rgb): Oklab {
		const r = srgbToLinear(color.r);
		const g = srgbToLinear(color.g);
		const b = srgbToLinear(color.b);
		const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
		const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
		const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
		return {
			l: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
			a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
			b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
		};
	}

	function oklabToRgb(lab: Oklab): Rgb {
		const l = (lab.l + 0.3963377774 * lab.a + 0.2158037573 * lab.b) ** 3;
		const m = (lab.l - 0.1055613458 * lab.a - 0.0638541728 * lab.b) ** 3;
		const s = (lab.l - 0.0894841775 * lab.a - 1.291485548 * lab.b) ** 3;
		return {
			r: linearToSrgb(+4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
			g: linearToSrgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
			b: linearToSrgb(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s)
		};
	}

	function oklabToOklch(lab: Oklab): Oklch {
		return {
			l: lab.l,
			c: Math.hypot(lab.a, lab.b),
			h: ((Math.atan2(lab.b, lab.a) * 180) / Math.PI + 360) % 360
		};
	}

	function oklchToOklab(lch: Oklch): Oklab {
		const radians = (lch.h * Math.PI) / 180;
		return { l: lch.l, a: Math.cos(radians) * lch.c, b: Math.sin(radians) * lch.c };
	}
</script>

<Dialog bind:open>
	<DialogContent class="max-w-2xl">
		<DialogHeader>
			<DialogTitle>{title()}</DialogTitle>
			<DialogDescription>
				Pick visually, type exact values, and attach searchable tags.
			</DialogDescription>
		</DialogHeader>

		<div class="grid gap-4 py-2">
			<div class="grid grid-cols-[4rem_minmax(0,1fr)] items-center gap-3">
				<span
					class="block size-16 border border-border"
					style="background-color: {rgbFromHex(hex) ? hex : '#000000'}"
					aria-hidden="true"
				></span>
				<div class="grid gap-2">
					<Label for="palette-color-name">Name</Label>
					<Input id="palette-color-name" bind:value={name} placeholder="Sky blue" />
				</div>
			</div>

			<Select bind:value={picker} type="single">
				<SelectTrigger size="sm" class="w-full" aria-label="Color picker mode">
					{pickerOptions.find((option) => option.id === picker)?.label ?? 'Picker'}
				</SelectTrigger>
				<SelectContent>
					{#each pickerOptions as option (option.id)}
						<SelectItem value={option.id}>{option.label}</SelectItem>
					{/each}
				</SelectContent>
			</Select>

			{#if sliderChannels.length > 0}
				<ColorSliderPicker channels={sliderChannels} />
			{/if}

			{#if picker === 'hsl-wheel'}
				<ColorWheelPicker
					{hueWheelBackground}
					hue={hsv.h}
					hueHandleStyle={hueWheelHandleStyle}
					{triangleHandleStyle}
					onPick={pickFromWheel}
				/>
			{:else if picker === 'oklab'}
				<ColorSpacePreviewPicker
					description="OKLab replaces the RGB hex-byte fields for perceptual editing."
					values={oklabValues}
				/>
			{:else if picker === 'oklch'}
				<ColorSpacePreviewPicker
					description="OKLCH replaces CMYK for lightness/chroma/hue adjustments."
					values={oklchValues}
				/>
			{:else}
				<ColorPlanePicker
					fieldBackground={planeBackground}
					handleStyle={planeHandleStyle}
					onPickPlane={pickFromPlane}
				/>
			{/if}

			<div class="grid gap-3 text-xs">
				<div class="grid grid-cols-2 gap-x-5 gap-y-1">
					{@render numberInput(
						'R',
						rgb.r,
						0,
						255,
						(value) => setColorFromRgb({ ...rgb, r: value }),
						1,
						0,
						'col-start-1 row-start-1'
					)}
					{@render numberInput(
						'G',
						rgb.g,
						0,
						255,
						(value) => setColorFromRgb({ ...rgb, g: value }),
						1,
						0,
						'col-start-1 row-start-2'
					)}
					{@render numberInput(
						'B',
						rgb.b,
						0,
						255,
						(value) => setColorFromRgb({ ...rgb, b: value }),
						1,
						0,
						'col-start-1 row-start-3'
					)}
					{@render numberInput(
						'H',
						hsl.h,
						0,
						360,
						(value) => updateHsl({ h: value }),
						1,
						0,
						'col-start-2 row-start-1'
					)}
					{@render numberInput(
						'S',
						hsl.s,
						0,
						100,
						(value) => updateHsl({ s: value }),
						1,
						0,
						'col-start-2 row-start-2'
					)}
					{@render numberInput(
						'L',
						hsl.l,
						0,
						100,
						(value) => updateHsl({ l: value }),
						1,
						0,
						'col-start-2 row-start-3'
					)}
				</div>
				<div class="grid grid-cols-2 gap-x-5 gap-y-1">
					{@render numberInput(
						'L',
						oklab.l,
						0,
						1,
						(value) => updateOklab({ l: value }),
						0.001,
						3,
						'col-start-1 row-start-1'
					)}
					{@render numberInput(
						'a',
						oklab.a,
						-0.4,
						0.4,
						(value) => updateOklab({ a: value }),
						0.001,
						3,
						'col-start-1 row-start-2'
					)}
					{@render numberInput(
						'b',
						oklab.b,
						-0.4,
						0.4,
						(value) => updateOklab({ b: value }),
						0.001,
						3,
						'col-start-1 row-start-3'
					)}
					{@render numberInput(
						'L',
						oklch.l,
						0,
						1,
						(value) => updateOklch({ l: value }),
						0.001,
						3,
						'col-start-2 row-start-1'
					)}
					{@render numberInput(
						'C',
						oklch.c,
						0,
						0.4,
						(value) => updateOklch({ c: value }),
						0.001,
						3,
						'col-start-2 row-start-2'
					)}
					{@render numberInput(
						'H',
						oklch.h,
						0,
						360,
						(value) => updateOklch({ h: value }),
						1,
						0,
						'col-start-2 row-start-3'
					)}
				</div>
				<div class="grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-2">
					<Label for="palette-color-hex" class="text-xs text-muted-foreground">Hex</Label>
					<Input id="palette-color-hex" bind:value={hex} placeholder="#66AAFF" />
				</div>
			</div>

			<div class="grid gap-2">
				<Label for="palette-color-tag">Tags</Label>
				<div class="flex gap-2">
					<Input
						id="palette-color-tag"
						bind:value={tagDraft}
						placeholder="free, ramp, skin-tone…"
						onkeydown={(event) => {
							if (event.key !== 'Enter') return;
							event.preventDefault();
							addTag();
						}}
					/>
					<Button variant="outline" onclick={addTag}>Add</Button>
				</div>
				<div class="flex flex-wrap gap-1">
					{#each tags as tag (tag)}
						<button
							type="button"
							class="border border-border bg-muted px-2 py-1 text-xs hover:bg-muted/70"
							onclick={() => removeTag(tag)}
							aria-label="Remove tag {tag}"
						>
							{tag} ×
						</button>
					{/each}
				</div>
			</div>
		</div>

		<DialogFooter>
			<Button variant="outline" onclick={() => (open = false)}>Cancel</Button>
			<Button onclick={onSave}>Save color</Button>
		</DialogFooter>
	</DialogContent>
</Dialog>

{#snippet numberInput(
	label: string,
	value: number,
	min: number,
	max: number,
	onChange: (value: number) => void,
	step = 1,
	digits = 0,
	wrapperClass = ''
)}
	<label class="grid grid-cols-[3rem_minmax(0,1fr)] items-center gap-1 {wrapperClass}">
		<span class="text-muted-foreground">{label}</span>
		<input
			class="h-7 border border-input bg-background px-1 text-right font-mono text-xs tabular-nums"
			type="number"
			{min}
			{max}
			{step}
			value={value.toFixed(digits)}
			onchange={(event) => onChange(event.currentTarget.valueAsNumber)}
		/>
	</label>
{/snippet}
