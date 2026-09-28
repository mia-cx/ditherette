<script lang="ts">
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Label } from '$lib/components/ui/label';
	import {
		Select,
		SelectContent,
		SelectItem,
		SelectSeparator,
		SelectTrigger
	} from '$lib/components/ui/select';
	import type { ColourChannel } from 'ditherette';
	import {
		CURVE_MODELS,
		FLAT,
		STRAIGHT,
		XY_CHANNELS,
		samePoints,
		type ChannelName,
		type CurveModel,
		type CurvePoints,
		type CurvesLayer
	} from '$lib/effects/catalog';
	import { hueAxis } from '$lib/effects/tone';
	import CurveGraph from './CurveGraph.svelte';

	type Props = { id: string; step: CurvesLayer; onchange: (step: CurvesLayer) => void };
	let { id, step, onchange }: Props = $props();

	type Channel = 0 | 1 | 2;
	const CHANNELS: readonly Channel[] = [0, 1, 2];
	const XY = 'xy';

	/** Each channel's colour, as a stroke and as its checkbox fill. */
	const TONE: Record<ChannelName, { stroke: string; check: string }> = {
		red: { stroke: 'text-red-500', check: 'data-checked:border-red-500 data-checked:bg-red-500' },
		green: {
			stroke: 'text-green-500',
			check: 'data-checked:border-green-500 data-checked:bg-green-500'
		},
		blue: {
			stroke: 'text-blue-500',
			check: 'data-checked:border-blue-500 data-checked:bg-blue-500'
		},
		hue: {
			stroke: 'text-amber-500',
			check: 'data-checked:border-amber-500 data-checked:bg-amber-500'
		},
		saturation: {
			stroke: 'text-fuchsia-500',
			check: 'data-checked:border-fuchsia-500 data-checked:bg-fuchsia-500'
		},
		chroma: {
			stroke: 'text-fuchsia-500',
			check: 'data-checked:border-fuchsia-500 data-checked:bg-fuchsia-500'
		},
		lightness: { stroke: 'text-foreground', check: '' },
		value: { stroke: 'text-foreground', check: '' },
		luma: { stroke: 'text-foreground', check: '' },
		a: {
			stroke: 'text-emerald-500',
			check: 'data-checked:border-emerald-500 data-checked:bg-emerald-500'
		},
		b: { stroke: 'text-sky-500', check: 'data-checked:border-sky-500 data-checked:bg-sky-500' },
		cb: { stroke: 'text-sky-500', check: 'data-checked:border-sky-500 data-checked:bg-sky-500' },
		cr: { stroke: 'text-rose-500', check: 'data-checked:border-rose-500 data-checked:bg-rose-500' }
	};

	const modelOf = (model: CurveModel) => CURVE_MODELS.find((candidate) => candidate.id === model)!;
	/** RGB edits all three channels together at first; other models start on their lightness. */
	function defaultEditing(model: CurveModel): Channel[] {
		if (model === 'srgb' || model === 'linear-rgb') return [...CHANNELS];
		const { channels } = modelOf(model);
		return CHANNELS.filter((channel) =>
			['lightness', 'value', 'luma'].includes(channels[channel].name)
		);
	}

	/** Checked channels. Edits start from the first one's curve and write to all of them. */
	// svelte-ignore state_referenced_locally
	let editing = $state<Channel[]>(defaultEditing(step.model === XY ? 'srgb' : step.model));

	const modelLabel = $derived(step.model === XY ? 'Arbitrary XY' : modelOf(step.model).label);

	/** Hue versus saturation, the classic targeted adjustment, until someone picks other channels. */
	const DEFAULT_XY = {
		x: { model: 'hsl', channel: 'hue' },
		y: { model: 'hsl', channel: 'saturation' }
	} as const satisfies Record<'x' | 'y', ColourChannel>;
	const channelKey = ({ model, channel }: ColourChannel) => `${model}:${channel}`;
	const channelLabel = (channel: ColourChannel) =>
		XY_CHANNELS.find((option) => channelKey(option.channel) === channelKey(channel))!.label;

	/** Switching models starts over: a curve drawn for one model's channels means something else in another. */
	function setModel(model: string) {
		if (model === step.model) return;
		if (model === XY) {
			onchange({ effect: 'curves', enabled: step.enabled, model: XY, ...DEFAULT_XY, points: FLAT });
			return;
		}
		const next = CURVE_MODELS.find((candidate) => candidate.id === model);
		if (!next) return;
		editing = defaultEditing(next.id);
		onchange({
			effect: 'curves',
			enabled: step.enabled,
			model: next.id,
			curves: [STRAIGHT, STRAIGHT, STRAIGHT]
		});
	}

	function setAxis(axis: 'x' | 'y', key: string) {
		if (step.model !== XY) return;
		const option = XY_CHANNELS.find(({ channel }) => channelKey(channel) === key);
		if (!option) return;
		// A curve drawn over one input means nothing over another, so a new x starts flat, like a new
		// model. A new y keeps the curve: it still adjusts wherever x puts it.
		onchange({ ...step, [axis]: option.channel, points: axis === 'x' ? FLAT : step.points });
	}

	function setEditing(channel: Channel, checked: boolean) {
		editing = CHANNELS.filter((other) => (other === channel ? checked : editing.includes(other)));
	}
</script>

<div class="grid grid-cols-1 gap-3">
	<div class="grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2">
		<Label for="{id}-model" class="text-xs text-muted-foreground">Model</Label>
		<Select type="single" value={step.model} onValueChange={setModel}>
			<SelectTrigger id="{id}-model" class="w-full">{modelLabel}</SelectTrigger>
			<SelectContent>
				{#each CURVE_MODELS as model (model.id)}
					<SelectItem value={model.id}>{model.label}</SelectItem>
				{/each}
				<SelectSeparator />
				<SelectItem value={XY}>Arbitrary XY</SelectItem>
			</SelectContent>
		</Select>
	</div>

	{#if step.model === XY}
		{@const hue = step.x.channel === 'hue' ? step.x.model : undefined}
		{#each [{ axis: 'x', label: 'X reads', channel: step.x }, { axis: 'y', label: 'Y adjusts', channel: step.y }] as const as { axis, label, channel } (axis)}
			<div class="grid grid-cols-[6rem_minmax(0,1fr)] items-center gap-2">
				<Label for="{id}-{axis}" class="text-xs text-muted-foreground">{label}</Label>
				<Select
					type="single"
					value={channelKey(channel)}
					onValueChange={(key) => setAxis(axis, key)}
				>
					<SelectTrigger id="{id}-{axis}" class="w-full">{channelLabel(channel)}</SelectTrigger>
					<SelectContent class="max-h-80">
						{#each XY_CHANNELS as option (channelKey(option.channel))}
							<SelectItem value={channelKey(option.channel)}>{option.label}</SelectItem>
						{/each}
					</SelectContent>
				</Select>
			</div>
		{/each}
		<CurveGraph
			{id}
			points={step.points}
			stroke={TONE[step.y.channel].stroke}
			neutral="flat"
			periodic={hue !== undefined}
			spectrum={hue ? hueAxis(hue) : undefined}
			axes={{ x: channelLabel(step.x), y: 'Adjustment' }}
			onchange={(points: CurvePoints) => onchange({ ...step, points })}
		/>
	{:else}
		{@const channels = modelOf(step.model).channels}
		{@const curves = step.curves}
		{@const points = curves[editing[0]!]}
		<div
			class="flex flex-wrap items-center gap-x-4 gap-y-2"
			role="group"
			aria-label="Channels to edit"
		>
			{#each CHANNELS as channel (channel)}
				<div class="flex items-center gap-2">
					<Checkbox
						id="{id}-{channel}"
						class={TONE[channels[channel].name].check}
						bind:checked={
							() => editing.includes(channel), (checked) => setEditing(channel, checked)
						}
						disabled={editing.length === 1 && editing[0] === channel}
					/>
					<Label for="{id}-{channel}" class="text-xs">{channels[channel].label}</Label>
				</div>
			{/each}
		</div>
		<CurveGraph
			{id}
			{points}
			stroke={editing.length === 1 ? TONE[channels[editing[0]!].name].stroke : 'text-foreground'}
			behind={CHANNELS.filter((channel) => !samePoints(curves[channel], points)).map((channel) => ({
				points: curves[channel],
				stroke: TONE[channels[channel].name].stroke
			}))}
			onchange={(next: CurvePoints) => {
				const pick = (channel: Channel) => (editing.includes(channel) ? next : curves[channel]);
				onchange({ ...step, curves: [pick(0), pick(1), pick(2)] });
			}}
		/>
	{/if}
</div>
