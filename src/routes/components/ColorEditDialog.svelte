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

	type PickerMode = 'spectrum' | 'wheel' | 'oklab';
	type Rgb = { r: number; g: number; b: number };
	type Hsv = { h: number; s: number; v: number };
	type Oklab = { l: number; a: number; b: number };
	type Props = {
		open: boolean;
		mode: 'add' | 'edit' | 'duplicate';
		name: string;
		hex: string;
		tags: string[];
		onSave: () => void;
	};

	let {
		open = $bindable(),
		mode,
		name = $bindable(),
		hex = $bindable(),
		tags = $bindable(),
		onSave
	}: Props = $props();

	let picker = $state<PickerMode>('spectrum');
	let tagDraft = $state('');
	let hsv = $state<Hsv>({ h: 0, s: 100, v: 100 });
	let oklab = $state<Oklab>({ l: 0.7, a: 0, b: 0 });
	let syncingFromPicker = false;

	const triangleRadiusRatio = 0.25;

	const selectorBackground = $derived(
		`linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl(${hsv.h} 100% 50%))`
	);
	const wheelTriangleBackground = $derived(
		`linear-gradient(to right, transparent, hsl(${hsv.h} 100% 50%)), linear-gradient(to bottom, #fff, #000)`
	);
	const hueRailBackground =
		'linear-gradient(to bottom, #ff0000 0%, #ffff00 16.666%, #00ff00 33.333%, #00ffff 50%, #0000ff 66.666%, #ff00ff 83.333%, #ff0000 100%)';
	const hueWheelBackground =
		'conic-gradient(from 90deg, #ff0000 0deg, #ffff00 60deg, #00ff00 120deg, #00ffff 180deg, #0000ff 240deg, #ff00ff 300deg, #ff0000 360deg)';
	const triangleHandleStyle = $derived.by(() => {
		const point = trianglePointForHsv(hsv);
		return `left: ${point.x}%; top: ${point.y}%;`;
	});
	const hueWheelHandleStyle = $derived.by(() => {
		const point = wheelPointForHue(hsv.h);
		return `left: ${point.x}%; top: ${point.y}%;`;
	});

	$effect(() => {
		if (syncingFromPicker) return;
		const rgb = rgbFromHex(hex);
		if (!rgb) return;
		hsv = rgbToHsv(rgb);
		oklab = rgbToOklab(rgb);
	});

	function setHexFromPicker(next: string) {
		syncingFromPicker = true;
		hex = next;
		queueMicrotask(() => (syncingFromPicker = false));
	}

	function updateHsv(patch: Partial<Hsv>) {
		const next = { ...hsv, ...patch };
		hsv = next;
		const rgb = hsvToRgb(next);
		setHexFromPicker(hexFromRgb(rgb));
		oklab = rgbToOklab(rgb);
	}

	function pickFromSquare(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const x = clamp((next.clientX - rect.left) / rect.width, 0, 1);
			const y = clamp((next.clientY - rect.top) / rect.height, 0, 1);
			updateHsv({ s: x * 100, v: (1 - y) * 100 });
		};
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => (target.onpointermove = null);
	}

	function pickFromHueRail(event: PointerEvent) {
		const target = event.currentTarget as HTMLElement;
		const update = (next: PointerEvent) => {
			const rect = target.getBoundingClientRect();
			const y = clamp((next.clientY - rect.top) / rect.height, 0, 1);
			updateHsv({ h: y * 360 });
		};
		target.setPointerCapture(event.pointerId);
		update(event);
		target.onpointermove = update;
		target.onpointerup = () => (target.onpointermove = null);
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
		return ((Math.atan2(y, x) * 180) / Math.PI - 90 + 360) % 360;
	}

	function wheelPointForHue(hue: number) {
		const angle = ((hue + 90) * Math.PI) / 180;
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

	function barycentric(
		point: { x: number; y: number },
		white: { x: number; y: number },
		black: { x: number; y: number },
		hue: { x: number; y: number }
	) {
		const denom = (black.y - hue.y) * (white.x - hue.x) + (hue.x - black.x) * (white.y - hue.y);
		const whiteWeight =
			((black.y - hue.y) * (point.x - hue.x) + (hue.x - black.x) * (point.y - hue.y)) / denom;
		const blackWeight =
			((hue.y - white.y) * (point.x - hue.x) + (white.x - hue.x) * (point.y - hue.y)) / denom;
		return {
			white: whiteWeight,
			black: blackWeight,
			hue: 1 - whiteWeight - blackWeight
		};
	}

	function closestPointInTriangle(
		point: { x: number; y: number },
		white: { x: number; y: number },
		black: { x: number; y: number },
		hue: { x: number; y: number }
	) {
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

	function closestPointOnSegment(
		point: { x: number; y: number },
		start: { x: number; y: number },
		end: { x: number; y: number }
	) {
		const dx = end.x - start.x;
		const dy = end.y - start.y;
		const lengthSquared = dx * dx + dy * dy;
		const t =
			lengthSquared === 0
				? 0
				: clamp(((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared, 0, 1);
		return { x: start.x + t * dx, y: start.y + t * dy };
	}

	function rotatePoint(point: { x: number; y: number }, degrees: number) {
		const radians = (degrees * Math.PI) / 180;
		const cos = Math.cos(radians);
		const sin = Math.sin(radians);
		return { x: point.x * cos - point.y * sin, y: point.x * sin + point.y * cos };
	}

	function distanceSquared(left: { x: number; y: number }, right: { x: number; y: number }) {
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

	function hexFromRgb(rgb: Rgb) {
		return `#${componentToHex(rgb.r)}${componentToHex(rgb.g)}${componentToHex(rgb.b)}`;
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

	function rgbToHsv(rgb: Rgb): Hsv {
		const r = rgb.r / 255;
		const g = rgb.g / 255;
		const b = rgb.b / 255;
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
		const [rp, gp, bp] =
			h < 60
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
		return { r: (rp + m) * 255, g: (gp + m) * 255, b: (bp + m) * 255 };
	}

	function srgbToLinear(value: number) {
		const channel = value / 255;
		return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
	}

	function rgbToOklab(rgb: Rgb): Oklab {
		const r = srgbToLinear(rgb.r);
		const g = srgbToLinear(rgb.g);
		const b = srgbToLinear(rgb.b);
		const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
		const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
		const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
		return {
			l: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
			a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
			b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s
		};
	}
</script>

<Dialog bind:open>
	<DialogContent class="max-w-xl">
		<DialogHeader>
			<DialogTitle>{title()}</DialogTitle>
			<DialogDescription>
				Pick visually, type a hex value directly, and attach searchable tags.
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

			<div class="flex flex-wrap gap-1">
				{#each [['spectrum', 'Gradient'], ['wheel', 'Hue wheel'], ['oklab', 'OKLab later']] as [id, label] (id)}
					<Button
						variant={picker === id ? 'secondary' : 'outline'}
						size="sm"
						onclick={() => (picker = id as PickerMode)}>{label}</Button
					>
				{/each}
			</div>

			{#if picker === 'spectrum'}
				<div
					class="grid grid-cols-[minmax(0,1fr)_1.75rem] gap-3 border border-border bg-muted/30 p-3"
				>
					<button
						type="button"
						class="relative aspect-square min-h-52 touch-none overflow-hidden border border-border bg-transparent p-0"
						aria-label="Saturation and value color field"
						onpointerdown={pickFromSquare}
					>
						<span class="absolute -inset-px" style="background: {selectorBackground};"></span>
						<span
							class="absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
							style="left: {hsv.s}%; top: {100 - hsv.v}%;"
						></span>
					</button>
					<button
						type="button"
						class="relative touch-none overflow-hidden border border-border bg-transparent p-0"
						aria-label="Hue rail"
						onpointerdown={pickFromHueRail}
					>
						<span class="absolute -inset-px" style="background: {hueRailBackground};"></span>
						<span
							class="absolute left-1/2 h-1.5 w-8 -translate-x-1/2 -translate-y-1/2 border border-background bg-foreground shadow-sm"
							style="top: {(hsv.h / 360) * 100}%;"
						></span>
					</button>
				</div>
			{:else if picker === 'wheel'}
				<div class="grid place-items-center border border-border bg-muted/30 p-4">
					<button
						type="button"
						class="relative size-72 touch-none overflow-hidden rounded-full border-0 bg-transparent p-0"
						aria-label="Hue wheel with saturation and value triangle"
						onpointerdown={pickFromWheel}
					>
						<span class="absolute -inset-px rounded-full" style="background: {hueWheelBackground};"
						></span>
						<span class="absolute inset-[13%] rounded-full bg-background"></span>
						<span
							class="absolute top-1/2 left-1/2 size-36 overflow-hidden [clip-path:polygon(100%_50%,25%_6.699%,25%_93.301%)]"
							style="transform: translate(-50%, -50%) rotate({hsv.h}deg);"
						>
							<span class="absolute -inset-px" style="background: {wheelTriangleBackground};"
							></span>
						</span>
						<span
							class="absolute size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
							style={hueWheelHandleStyle}
						></span>
						<span
							class="absolute size-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-white shadow-[0_0_0_1px_rgba(0,0,0,0.7)]"
							style={triangleHandleStyle}
						></span>
					</button>
				</div>
			{:else}
				<div class="grid gap-3 border border-border bg-muted/30 p-4 text-sm">
					<p class="text-muted-foreground">
						OKLab needs its own purpose-built picker, not another pile of sliders. Current color:
					</p>
					<div class="grid grid-cols-3 gap-2 font-mono text-xs tabular-nums">
						<span>L {oklab.l.toFixed(3)}</span>
						<span>a {oklab.a.toFixed(3)}</span>
						<span>b {oklab.b.toFixed(3)}</span>
					</div>
				</div>
			{/if}

			<div class="grid gap-1.5">
				<Label for="palette-color-hex">Hex</Label>
				<Input id="palette-color-hex" bind:value={hex} placeholder="#66AAFF" />
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
