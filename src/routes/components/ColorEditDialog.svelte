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

	type PickerMode = 'hsl' | 'hue-triangle' | 'oklab';
	type Rgb = { r: number; g: number; b: number };
	type Hsl = { h: number; s: number; l: number };
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

	let picker = $state<PickerMode>('hsl');
	let tagDraft = $state('');
	let hsl = $state<Hsl>({ h: 0, s: 100, l: 50 });
	let sv = $state({ h: 0, s: 100, v: 100 });
	let oklab = $state<Oklab>({ l: 0.7, a: 0, b: 0 });
	let syncingFromHex = false;

	$effect(() => {
		if (syncingFromHex) return;
		const rgb = rgbFromHex(hex);
		if (!rgb) return;
		hsl = rgbToHsl(rgb);
		sv = rgbToHsv(rgb);
		oklab = rgbToOklab(rgb);
	});

	function setHex(next: string) {
		syncingFromHex = true;
		hex = next;
		queueMicrotask(() => (syncingFromHex = false));
	}

	function updateHsl(patch: Partial<Hsl>) {
		hsl = { ...hsl, ...patch };
		setHex(hexFromRgb(hslToRgb(hsl)));
	}

	function updateSv(patch: Partial<typeof sv>) {
		sv = { ...sv, ...patch };
		setHex(hexFromRgb(hsvToRgb(sv)));
	}

	function updateOklab(patch: Partial<Oklab>) {
		oklab = { ...oklab, ...patch };
		setHex(hexFromRgb(oklabToRgb(oklab)));
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

	function rgbToHsl({ r, g, b }: Rgb): Hsl {
		r /= 255;
		g /= 255;
		b /= 255;
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

	function rgbToHsv(rgb: Rgb) {
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

	function hsvToRgb({ h, s, v }: typeof sv): Rgb {
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

	function linearToSrgb(value: number) {
		const channel = value <= 0.0031308 ? 12.92 * value : 1.055 * value ** (1 / 2.4) - 0.055;
		return clamp(channel * 255, 0, 255);
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
				{#each [['hsl', 'HSL'], ['hue-triangle', 'Hue circle + SV'], ['oklab', 'OKLab']] as [id, label] (id)}
					<Button
						variant={picker === id ? 'secondary' : 'outline'}
						size="sm"
						onclick={() => (picker = id as PickerMode)}>{label}</Button
					>
				{/each}
			</div>

			{#if picker === 'hsl'}
				<div class="grid gap-3 border border-border bg-muted/30 p-3">
					<input
						class="h-12 w-full"
						type="color"
						value={hex}
						oninput={(e) => setHex(e.currentTarget.value.toUpperCase())}
					/>
					{@render slider('Hue', hsl.h, 0, 360, (value) => updateHsl({ h: value }), '°')}
					{@render slider('Saturation', hsl.s, 0, 100, (value) => updateHsl({ s: value }), '%')}
					{@render slider('Lightness', hsl.l, 0, 100, (value) => updateHsl({ l: value }), '%')}
				</div>
			{:else if picker === 'hue-triangle'}
				<div class="grid gap-3 border border-border bg-muted/30 p-3">
					<div class="grid place-items-center py-2">
						<div
							class="grid size-36 place-items-center rounded-full border border-border bg-[conic-gradient(red,yellow,lime,cyan,blue,magenta,red)]"
						>
							<div
								class="size-20 border border-background/70 shadow-sm [clip-path:polygon(50%_0,0_100%,100%_100%)]"
								style="background: linear-gradient(to top, #000, transparent), linear-gradient(to right, #fff, hsl({sv.h} 100% 50%));"
							></div>
						</div>
					</div>
					{@render slider('Hue', sv.h, 0, 360, (value) => updateSv({ h: value }), '°')}
					{@render slider('Saturation', sv.s, 0, 100, (value) => updateSv({ s: value }), '%')}
					{@render slider('Value', sv.v, 0, 100, (value) => updateSv({ v: value }), '%')}
				</div>
			{:else}
				<div class="grid gap-3 border border-border bg-muted/30 p-3">
					<p class="text-xs text-muted-foreground">
						OKLab edits lightness separately from perceptual color axes, which is useful for palette
						ramps.
					</p>
					{@render slider(
						'Lightness',
						oklab.l,
						0,
						1,
						(value) => updateOklab({ l: value }),
						'',
						0.01
					)}
					{@render slider(
						'Green ↔ Red',
						oklab.a,
						-0.4,
						0.4,
						(value) => updateOklab({ a: value }),
						'',
						0.01
					)}
					{@render slider(
						'Blue ↔ Yellow',
						oklab.b,
						-0.4,
						0.4,
						(value) => updateOklab({ b: value }),
						'',
						0.01
					)}
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

{#snippet slider(
	label: string,
	value: number,
	min: number,
	max: number,
	onChange: (value: number) => void,
	suffix = '',
	step = 1
)}
	<label class="grid grid-cols-[6rem_minmax(0,1fr)_4rem] items-center gap-2 text-xs">
		<span class="text-muted-foreground">{label}</span>
		<input
			type="range"
			{min}
			{max}
			{step}
			{value}
			oninput={(event) => onChange(event.currentTarget.valueAsNumber)}
		/>
		<span class="text-right font-mono tabular-nums">{value.toFixed(step < 1 ? 2 : 0)}{suffix}</span>
	</label>
{/snippet}
